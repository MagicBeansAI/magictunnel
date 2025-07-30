# Deployment Guide

## Overview

MagicTunnel provides flexible deployment options for different environments and scale requirements.

## Quick Start

### Local Development
```bash
# Clone and build
git clone https://github.com/gouravd/magictunnel
cd magictunnel
cargo build --release

# Run with default configuration
./target/release/magictunnel --config config.yaml

# Run in stdio mode for Claude Desktop
./target/release/magictunnel --stdio --config config.yaml

# Connect client to ws://localhost:3000/mcp/ws
```

### Docker
```bash
# Build Docker image
docker build -t magictunnel .

# Run with configuration
docker run -p 3000:3000 -v $(pwd)/config.yaml:/app/config.yaml magictunnel
```

## Configuration

### Basic Configuration (config.yaml)
```yaml
server:
  host: "0.0.0.0"
  port: 3000
  websocket: true
  timeout: 30

registry:
  type: "file"
  paths:
    - "./capabilities"
  hot_reload: true

external_mcp:
  enabled: true
  config_file: "external-mcp-servers.yaml"
  capabilities_output_dir: "./capabilities/external-mcp"
  refresh_interval_minutes: 5

logging:
  level: "info"
  format: "json"
```

### Environment Variables
Override any configuration setting using environment variables:

```bash
# Server settings
export MCP_HOST="0.0.0.0"
export MCP_PORT="8080"
export MCP_WEBSOCKET="true"
export MCP_TIMEOUT="60"

# Registry settings
export MCP_REGISTRY_TYPE="file"
export MCP_REGISTRY_PATHS="./capabilities,./tools"
export MCP_HOT_RELOAD="true"

# Logging settings
export MCP_LOG_LEVEL="debug"
export MCP_LOG_FORMAT="json"
```

## Docker Deployment

### Dockerfile
```dockerfile
FROM rust:1.70 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/magictunnel .
COPY config.yaml .
COPY capabilities/ ./capabilities/
EXPOSE 3000 4000
CMD ["./magictunnel", "--config", "config.yaml"]
```

### Docker Compose
```yaml
version: '3.8'
services:
  magictunnel:
    build: .
    ports:
      - "3000:3000"  # HTTP/WebSocket
      - "4000:4000"  # gRPC
    volumes:
      - ./config.yaml:/app/config.yaml
      - ./capabilities:/app/capabilities
      - ./external-mcp-servers.yaml:/app/external-mcp-servers.yaml
    environment:
      - MCP_LOG_LEVEL=info
      - MCP_HOT_RELOAD=true
    restart: unless-stopped
```

## Kubernetes Deployment

### ConfigMap
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: magictunnel-config
data:
  config.yaml: |
    server:
      host: "0.0.0.0"
      port: 3000
      websocket: true
    registry:
      type: "file"
      paths: ["/app/capabilities"]
    logging:
      level: "info"
      format: "json"
```

### Deployment
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: magictunnel
spec:
  replicas: 3
  selector:
    matchLabels:
      app: magictunnel
  template:
    metadata:
      labels:
        app: magictunnel
    spec:
      containers:
      - name: magictunnel
        image: magictunnel:latest
        ports:
        - containerPort: 3000
        - containerPort: 4000
        volumeMounts:
        - name: config
          mountPath: /app/config.yaml
          subPath: config.yaml
        env:
        - name: MCP_LOG_LEVEL
          value: "info"
      volumes:
      - name: config
        configMap:
          name: magictunnel-config
```

### Service
```yaml
apiVersion: v1
kind: Service
metadata:
  name: magictunnel-service
spec:
  selector:
    app: magictunnel
  ports:
  - name: http
    port: 80
    targetPort: 3000
  - name: grpc
    port: 4000
    targetPort: 4000
  type: LoadBalancer
```

## Security Configuration

### TLS/SSL Setup
```yaml
server:
  tls_mode: "application"  # or "reverse_proxy"
  cert_file: "/path/to/cert.pem"
  key_file: "/path/to/key.pem"
  host: "0.0.0.0"
  port: 443
```

### CORS Configuration
```yaml
server:
  cors:
    allow_origins: ["https://app.example.com"]
    allow_methods: ["GET", "POST"]
    allow_headers: ["Content-Type", "Authorization"]
```

### Authentication Setup
```yaml
authentication:
  type: "api_key"
  api_keys:
    - "your-secret-key-here"
    
# Or OAuth 2.0
authentication:
  type: "oauth"
  providers:
    github:
      client_id: "your-client-id"
      client_secret: "your-client-secret"
```

## Load Balancing and High Availability

### Nginx Configuration
```nginx
upstream magictunnel {
    server 127.0.0.1:3000;
    server 127.0.0.1:3001;
    server 127.0.0.1:3002;
}

server {
    listen 80;
    server_name magictunnel.example.com;
    
    location / {
        proxy_pass http://magictunnel;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### HAProxy Configuration
```
global
    daemon

defaults
    mode http
    timeout connect 5000ms
    timeout client 50000ms
    timeout server 50000ms

frontend magictunnel_frontend
    bind *:80
    default_backend magictunnel_backend

backend magictunnel_backend
    balance roundrobin
    server mt1 127.0.0.1:3000 check
    server mt2 127.0.0.1:3001 check
    server mt3 127.0.0.1:3002 check
```

## Monitoring and Observability

### Health Checks
```bash
# Health check endpoint
curl http://localhost:3000/health

# Detailed status
curl http://localhost:3000/status
```

### Metrics Endpoint
```bash
curl http://localhost:3000/metrics
```

### Logging Configuration
```yaml
logging:
  level: "info"                    # debug|info|notice|warning|error|critical|alert|emergency
  format: "json"                   # json|text
  file: "/var/log/magictunnel.log"  # Optional file output
  rate_limit:
    enabled: true
    max_messages_per_minute: 100
```

### Prometheus Integration
```yaml
# Add to config.yaml
metrics:
  enabled: true
  endpoint: "/metrics"
  format: "prometheus"
```

### Grafana Dashboard
```json
{
  "dashboard": {
    "title": "MagicTunnel Metrics",
    "panels": [
      {
        "title": "Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(magictunnel_requests_total[5m])"
          }
        ]
      },
      {
        "title": "Response Time",
        "type": "graph", 
        "targets": [
          {
            "expr": "magictunnel_request_duration_seconds"
          }
        ]
      }
    ]
  }
}
```

## Production Best Practices

### Resource Limits
```yaml
# Docker resource limits
services:
  magictunnel:
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 2G
        reservations:
          cpus: '1.0'
          memory: 1G
```

### Security Hardening
```yaml
# Use non-root user
FROM debian:bookworm-slim
RUN useradd -r -s /bin/false magictunnel
USER magictunnel

# Limit capabilities
security:
  capabilities:
    drop:
      - ALL
    add:
      - NET_BIND_SERVICE
```

### Backup and Recovery
```bash
# Backup capability definitions
tar -czf capabilities-backup.tar.gz capabilities/

# Backup configuration
cp config.yaml config-backup.yaml

# Backup external MCP configuration
cp external-mcp-servers.yaml external-mcp-backup.yaml
```

### Log Rotation
```bash
# Using logrotate
/var/log/magictunnel.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 magictunnel magictunnel
    postrotate
        systemctl reload magictunnel
    endscript
}
```

## Environment-Specific Configurations

### Development
```yaml
server:
  host: "127.0.0.1"
  port: 3000
  
logging:
  level: "debug"
  format: "text"
  
registry:
  hot_reload: true
  
external_mcp:
  enabled: true
```

### Staging
```yaml
server:
  host: "0.0.0.0"
  port: 3000
  
logging:
  level: "info"
  format: "json"
  
registry:
  hot_reload: false
  
authentication:
  type: "api_key"
  api_keys: ["staging-key"]
```

### Production
```yaml
server:
  host: "0.0.0.0"
  port: 3000
  tls_mode: "application"
  cert_file: "/etc/ssl/certs/magictunnel.pem"
  key_file: "/etc/ssl/private/magictunnel.key"
  
logging:
  level: "info"
  format: "json"
  file: "/var/log/magictunnel.log"
  
registry:
  hot_reload: false
  
authentication:
  type: "oauth"
  providers:
    github:
      client_id: "${GITHUB_CLIENT_ID}"
      client_secret: "${GITHUB_CLIENT_SECRET}"

metrics:
  enabled: true
  endpoint: "/metrics"
```

## Performance Tuning

### Server Optimization
```yaml
server:
  worker_threads: 8      # Number of async worker threads
  max_connections: 1000  # Maximum concurrent connections
  keep_alive: 75         # Keep-alive timeout in seconds
  buffer_size: 8192      # I/O buffer size
```

### Registry Optimization
```yaml
registry:
  cache_size: 10000      # Maximum cached entries
  cache_ttl: 3600        # Cache TTL in seconds
  parallel_loading: true # Load capabilities in parallel
  compression: true      # Compress cached data
```

### Database Connection Pooling
```yaml
database:
  max_connections: 20
  min_connections: 5
  idle_timeout: 300
  max_lifetime: 3600
```

## Troubleshooting

### Common Issues

1. **Port binding errors**: Check if ports are already in use
2. **Permission errors**: Ensure proper file permissions
3. **Memory issues**: Monitor memory usage and adjust limits
4. **Connection timeouts**: Tune timeout settings
5. **SSL/TLS errors**: Verify certificate validity

### Debug Commands
```bash
# Check port usage
netstat -tulpn | grep :3000

# Monitor resource usage
htop

# Check logs
tail -f /var/log/magictunnel.log

# Test connectivity
curl -v http://localhost:3000/health
```

### Performance Monitoring
```bash
# Monitor system resources
iostat -x 1
vmstat 1
free -h

# Monitor network
iftop
nethogs

# Application-specific monitoring
curl http://localhost:3000/metrics
```

## Maintenance

### Updates
```bash
# Pull latest version
git pull origin main

# Build and restart
cargo build --release
systemctl restart magictunnel

# Verify deployment
curl http://localhost:3000/health
```

### Database Maintenance
```bash
# Backup database
pg_dump magictunnel > backup.sql

# Vacuum and analyze
psql -c "VACUUM ANALYZE;"

# Update statistics
psql -c "ANALYZE;"
```

### Log Cleanup
```bash
# Clean old logs
find /var/log -name "magictunnel*.log*" -mtime +30 -delete

# Compress large logs
gzip /var/log/magictunnel.log.1
```

This deployment guide provides comprehensive instructions for deploying MagicTunnel in various environments, from local development to enterprise production systems.