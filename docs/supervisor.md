# 🔧 MagicTunnel Supervisor Guide

The MagicTunnel Supervisor is a process management system that provides automated service lifecycle management, monitoring, and restart capabilities for the MagicTunnel ecosystem.

## Overview

The supervisor architecture provides:
- **Process Management**: Automatic startup, monitoring, and restart of MagicTunnel services
- **Health Monitoring**: Continuous health checks and process status tracking
- **Web Dashboard Integration**: Full integration with the web-based management interface
- **Graceful Shutdowns**: Proper service shutdown and resource cleanup
- **Restart Orchestration**: Custom restart workflows with pre/post commands

## Quick Start

```bash
# Build the supervisor
cargo build --release --bin magictunnel-supervisor

# Run the supervisor (starts all services)
./target/release/magictunnel-supervisor

# Access web dashboard
open http://localhost:5173/dashboard
```

## Architecture

### Service Hierarchy
```
┌─────────────────────────────────────────────────────────┐
│                MagicTunnel Supervisor                   │
│                     (Port 8081)                        │
│                                                         │
│  ┌─────────────────┐  ┌─────────────────┐             │
│  │ Process Manager │  │ TCP Control     │             │
│  │                 │  │ Server          │             │
│  └─────────────────┘  └─────────────────┘             │
│           │                    │                       │
│           ▼                    ▼                       │
│  ┌─────────────────┐  ┌─────────────────┐             │
│  │ Health Monitor  │  │ Command Handler │             │
│  │                 │  │                 │             │
│  └─────────────────┘  └─────────────────┘             │
└─────────────────────────────────────────────────────────┘
           │                    │
           ▼                    ▼
┌─────────────────┐    ┌─────────────────┐
│ MagicTunnel     │    │ Frontend Dev    │
│ Main Server     │    │ Server (Vite)   │
│ (Port 3001)     │    │ (Port 5173)     │
└─────────────────┘    └─────────────────┘
           │
           ▼
┌─────────────────┐
│ External MCP    │
│ Services        │
│ (Port 8082+)    │
└─────────────────┘
```

### Communication Flow
```
Web Dashboard (5173) 
    ↓ HTTP API Call
MagicTunnel API (3001)
    ↓ TCP Socket
Supervisor Process (8081)
    ↓ Process Control
Target Services (MagicTunnel, External MCP, etc.)
```

## Core Features

### 1. Process Lifecycle Management
- **Automatic Startup**: Starts MagicTunnel main server on supervisor launch
- **Health Monitoring**: Continuous process health checks with configurable intervals
- **Automatic Restart**: Restarts failed processes with exponential backoff
- **Graceful Shutdown**: Proper signal handling for clean process termination
- **Resource Cleanup**: Automatic cleanup of resources on process exit

### 2. TCP Control Server
The supervisor runs a TCP server on port 8081 for command and control:

```rust
// TCP Commands supported
pub enum SupervisorCommand {
    Status,           // Get current status of all processes
    Restart(String),  // Restart specific service
    Stop(String),     // Stop specific service  
    Start(String),    // Start specific service
    CustomRestart {   // Advanced restart with workflows
        pre_commands: Vec<Command>,
        post_commands: Vec<Command>,
        args: Vec<String>,
    },
    Shutdown,         // Shutdown supervisor and all services
}
```

### 3. Custom Restart Workflows
Advanced restart capabilities with pre/post command execution:

```json
{
  "pre_commands": [
    {
      "command_type": "make",
      "command": "clean",
      "is_safe": true
    }
  ],
  "post_commands": [
    {
      "command_type": "shell", 
      "command": "echo 'Restart complete'",
      "is_safe": true
    }
  ],
  "args": ["--config", "production.yaml"]
}
```

### 4. Health Monitoring System
- **Process Status**: Tracks PID, uptime, memory usage, CPU usage
- **Service Health**: Monitors service-specific health endpoints
- **Alert System**: Detects and reports service failures
- **Recovery Actions**: Automatic recovery procedures for common failures

## Configuration

### Supervisor Configuration
The supervisor can be configured via environment variables or command-line arguments:

```bash
# Environment Variables
export SUPERVISOR_PORT=8081
export SUPERVISOR_HOST=127.0.0.1
export MAGICTUNNEL_CONFIG=./config.yaml
export SUPERVISOR_LOG_LEVEL=info

# Command Line Arguments
./target/release/magictunnel-supervisor \
  --port 8081 \
  --config ./config.yaml \
  --log-level debug
```

### Service Configuration
Services managed by the supervisor are configured in the main configuration file:

```yaml
# config.yaml
supervisor:
  enabled: true
  port: 8081
  host: "127.0.0.1"
  
  # Services to manage
  services:
    magictunnel:
      command: "./target/release/magictunnel"
      args: ["--config", "config.yaml"]
      health_check_interval: 30
      restart_policy: "always"
      max_restarts: 10
      
    frontend:
      command: "npm"
      args: ["run", "dev"]
      working_dir: "./frontend"
      health_check_url: "http://localhost:5173"
      restart_policy: "on_failure"
      
  # Health monitoring
  health_check:
    interval: 30
    timeout: 10
    retries: 3
    
  # Restart policies
  restart:
    delay: 5000      # Initial delay in ms
    max_delay: 60000 # Maximum delay in ms
    backoff: 2.0     # Exponential backoff multiplier
```

## API Integration

### Web Dashboard Integration
The supervisor integrates seamlessly with the web dashboard:

```typescript
// Frontend API calls
const restartService = async (serviceName: string) => {
  const response = await fetch('/dashboard/api/system/restart', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ service: serviceName })
  });
  return response.json();
};

const getServiceStatus = async () => {
  const response = await fetch('/dashboard/api/system/status');
  return response.json();
};
```

### Custom Restart API
Advanced restart functionality via REST API:

```bash
# Custom restart with pre/post commands
curl -X POST http://localhost:3001/dashboard/api/system/custom-restart \
  -H "Content-Type: application/json" \
  -d '{
    "pre_commands": [
      {
        "command_type": "make",
        "command": "clean",
        "is_safe": true
      }
    ],
    "post_commands": [
      {
        "command_type": "shell",
        "command": "echo \"Restart complete\"",
        "is_safe": true
      }
    ],
    "args": ["--config", "production.yaml"]
  }'
```

## Command Line Interface

### Direct TCP Communication
You can communicate directly with the supervisor via TCP:

```bash
# Using netcat to send commands
echo "status" | nc localhost 8081

# Using telnet
telnet localhost 8081
> status
> restart magictunnel
> shutdown
```

### Supervisor CLI Tool
```bash
# Check supervisor status
./target/release/magictunnel-supervisor-cli status

# Restart specific service
./target/release/magictunnel-supervisor-cli restart magictunnel

# Custom restart workflow
./target/release/magictunnel-supervisor-cli custom-restart \
  --pre-command "make clean" \
  --post-command "echo done" \
  --args "--config production.yaml"
```

## Monitoring & Observability

### Process Metrics
The supervisor collects and exposes process metrics:

```json
{
  "supervisor": {
    "status": "running",
    "uptime": 3600,
    "managed_services": 3
  },
  "services": {
    "magictunnel": {
      "status": "running",
      "pid": 12345,
      "uptime": 3500,
      "memory_mb": 45.2,
      "cpu_percent": 2.1,
      "restart_count": 0
    }
  }
}
```

### Health Checks
Built-in health check system:

```rust
pub struct HealthCheck {
    pub service_name: String,
    pub endpoint: Option<String>,    // HTTP health check URL
    pub interval: Duration,          // Check interval
    pub timeout: Duration,           // Request timeout
    pub consecutive_failures: u32,   // Failures before restart
}
```

### Logging Integration
The supervisor integrates with the MagicTunnel logging system:

```rust
use tracing::{info, warn, error};

// Structured logging for supervisor events
info!(
    service = %service_name,
    pid = %process_id,
    uptime = %uptime_seconds,
    "Service health check passed"
);

warn!(
    service = %service_name,
    error = %error_message,
    "Service health check failed"
);
```

## Production Deployment

### Systemd Integration
Example systemd service file for production deployment:

```ini
[Unit]
Description=MagicTunnel Supervisor
After=network.target

[Service]
Type=simple
User=magictunnel
Group=magictunnel
WorkingDirectory=/opt/magictunnel
ExecStart=/opt/magictunnel/bin/magictunnel-supervisor
Restart=always
RestartSec=5
Environment=RUST_LOG=info
Environment=SUPERVISOR_PORT=8081

[Install]
WantedBy=multi-user.target
```

### Docker Deployment
```dockerfile
# Dockerfile for supervisor
FROM rust:1.70 AS builder
COPY . .
RUN cargo build --release --bin magictunnel-supervisor

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/magictunnel-supervisor /usr/local/bin/
EXPOSE 8081
CMD ["magictunnel-supervisor"]
```

## Security Considerations

### Access Control
```yaml
supervisor:
  security:
    bind_to_localhost: true  # Only bind to 127.0.0.1
    allowed_commands:        # Restrict available commands
      - "status"
      - "restart"
    authentication:
      enabled: true
      token: "${SUPERVISOR_TOKEN}"
```

### Command Safety
```rust
pub struct Command {
    pub command_type: CommandType,
    pub command: String,
    pub is_safe: bool,        // Prevents dangerous commands
    pub allowed_in_production: bool,
}
```

## Troubleshooting

### Common Issues

1. **Supervisor Won't Start**
   ```bash
   # Check port availability
   netstat -an | grep 8081
   
   # Check permissions
   ls -la target/release/magictunnel-supervisor
   
   # Check logs
   RUST_LOG=debug ./target/release/magictunnel-supervisor
   ```

2. **Service Won't Restart**
   ```bash
   # Check supervisor status
   curl http://localhost:3001/dashboard/api/system/status
   
   # Check process status
   ps aux | grep magictunnel
   
   # Check supervisor logs
   tail -f /var/log/magictunnel/supervisor.log
   ```

3. **TCP Connection Issues**
   ```bash
   # Test TCP connectivity
   telnet localhost 8081
   
   # Check firewall
   sudo ufw status
   
   # Check process listening
   lsof -i :8081
   ```

### Debug Mode
```bash
# Run supervisor with debug logging
RUST_LOG=debug ./target/release/magictunnel-supervisor

# Enable trace-level logging for supervisor module
RUST_LOG=magictunnel::supervisor=trace ./target/release/magictunnel-supervisor
```

The MagicTunnel Supervisor transforms MagicTunnel from a simple service into a robust, production-ready platform with enterprise-grade process management, monitoring, and control capabilities.