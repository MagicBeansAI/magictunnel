# TLS Deployment Guide for MCP Proxy

## ✅ Implementation Status

**Phase 3.4.1 Complete** - All TLS deployment scenarios are fully implemented and tested:

- ✅ **Application Mode**: Direct HTTPS with rustls - Production ready
- ✅ **BehindProxy Mode**: HTTP with reverse proxy TLS termination - Production ready
- ✅ **Auto Mode**: Smart detection of proxy headers - Production ready
- ✅ **Advanced Security**: HSTS, security headers, rate limiting, DDoS protection
- ✅ **Certificate Monitoring**: Automated health checks and expiration alerts
- ✅ **Comprehensive Testing**: 47 TLS and security tests validating all deployment scenarios

## Quick Start

### Development Setup (Self-Signed Certificates)
```bash
# Generate self-signed certificate for development
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes \
  -subj "/C=US/ST=State/L=City/O=Organization/CN=localhost"

# Configure MCP Proxy
cat > config.yaml << EOF
server:
  host: "127.0.0.1"
  port: 3000
  tls:
    mode: "application"
    cert_file: "cert.pem"
    key_file: "key.pem"
EOF

# Start MCP Proxy
./magictunnel --config config.yaml

# Test HTTPS connection
curl -k https://localhost:3000/health
```

### Production Setup (Let's Encrypt)
```bash
# Install certbot
sudo apt-get install certbot

# Generate certificate
sudo certbot certonly --standalone -d magictunnel.example.com

# Configure MCP Proxy
cat > config.yaml << EOF
server:
  host: "0.0.0.0"
  port: 3000
  tls:
    mode: "application"
    cert_file: "/etc/letsencrypt/live/magictunnel.example.com/fullchain.pem"
    key_file: "/etc/letsencrypt/live/magictunnel.example.com/privkey.pem"
    min_tls_version: "1.2"
    hsts_enabled: true
EOF
```

## Deployment Scenarios

### 1. Direct HTTPS Deployment

#### Use Cases
- Development environments
- Small-scale production
- Edge deployments
- When you need end-to-end encryption

#### Configuration
```yaml
server:
  host: "0.0.0.0"
  port: 3000
  tls:
    mode: "application"
    cert_file: "/etc/ssl/certs/server.crt"
    key_file: "/etc/ssl/private/server.key"
    ca_file: "/etc/ssl/certs/ca.crt"
    min_tls_version: "1.2"
    cipher_suites:
      - "TLS_AES_256_GCM_SHA384"
      - "TLS_CHACHA20_POLY1305_SHA256"
      - "TLS_AES_128_GCM_SHA256"
    hsts_enabled: true
    hsts_max_age: 31536000
```

#### Client Connections
```bash
# HTTPS API calls
curl https://magictunnel.example.com:3000/mcp/tools

# Secure WebSocket connections
wscat -c wss://magictunnel.example.com:3000/mcp/ws
```

### 2. Behind Reverse Proxy (nginx)

#### Use Cases
- Production environments
- Load balancing
- Multiple services on same domain
- Certificate management at proxy level

#### nginx Configuration
```nginx
upstream mcp_proxy {
    server magictunnel:3000;
    # Add more servers for load balancing
    # server magictunnel-2:3000;
}

server {
    listen 80;
    server_name magictunnel.example.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name magictunnel.example.com;
    
    # SSL Configuration
    ssl_certificate /etc/ssl/certs/server.crt;
    ssl_certificate_key /etc/ssl/private/server.key;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512;
    ssl_prefer_server_ciphers off;
    
    # Security Headers
    add_header Strict-Transport-Security "max-age=63072000" always;
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    
    # Proxy Configuration
    location / {
        proxy_pass http://mcp_proxy;
        
        # Essential headers for MCP Proxy inspection
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-Host $host;
        proxy_set_header Authorization $http_authorization;
        
        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }
    
    # WebSocket Support
    location /mcp/ws {
        proxy_pass http://mcp_proxy;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # WebSocket timeouts
        proxy_read_timeout 86400;
    }
}
```

#### MCP Proxy Configuration
```yaml
server:
  host: "127.0.0.1"  # Internal only
  port: 3000
  tls:
    mode: "behind_proxy"
    behind_proxy: true
    trusted_proxies:
      - "10.0.0.0/8"
      - "172.16.0.0/12"
      - "192.168.0.0/16"
      - "127.0.0.1/32"
```

### 3. Traefik Reverse Proxy

#### docker-compose.yml
```yaml
version: '3.8'

services:
  traefik:
    image: traefik:v3.0
    command:
      - "--api.insecure=true"
      - "--providers.docker=true"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
      - "--certificatesresolvers.letsencrypt.acme.httpchallenge=true"
      - "--certificatesresolvers.letsencrypt.acme.httpchallenge.entrypoint=web"
      - "--certificatesresolvers.letsencrypt.acme.email=admin@example.com"
      - "--certificatesresolvers.letsencrypt.acme.storage=/letsencrypt/acme.json"
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - "/var/run/docker.sock:/var/run/docker.sock:ro"
      - "./letsencrypt:/letsencrypt"

  magictunnel:
    image: magictunnel:latest
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.magictunnel.rule=Host(`magictunnel.example.com`)"
      - "traefik.http.routers.magictunnel.entrypoints=websecure"
      - "traefik.http.routers.magictunnel.tls.certresolver=letsencrypt"
      - "traefik.http.services.magictunnel.loadbalancer.server.port=3000"
      # Middleware for headers
      - "traefik.http.middlewares.mcp-headers.headers.customrequestheaders.X-Forwarded-Proto=https"
      - "traefik.http.routers.magictunnel.middlewares=mcp-headers"
    volumes:
      - "./config.yaml:/app/config.yaml"
```

### 4. Kubernetes Deployment

#### Certificate Management with cert-manager
```yaml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: magictunnel-tls
  namespace: default
spec:
  secretName: magictunnel-tls-secret
  issuerRef:
    name: letsencrypt-prod
    kind: ClusterIssuer
  dnsNames:
  - magictunnel.example.com
```

#### Ingress Configuration
```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: magictunnel-ingress
  annotations:
    kubernetes.io/ingress.class: nginx
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "86400"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "86400"
    # WebSocket support
    nginx.ingress.kubernetes.io/websocket-services: magictunnel-service
spec:
  tls:
  - hosts:
    - magictunnel.example.com
    secretName: magictunnel-tls-secret
  rules:
  - host: magictunnel.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: magictunnel-service
            port:
              number: 3000
```

#### Deployment with Auto Mode
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
        env:
        - name: MCP_TLS_MODE
          value: "auto"
        - name: MCP_TLS_TRUSTED_PROXIES
          value: "10.0.0.0/8,172.16.0.0/12"
        volumeMounts:
        - name: config
          mountPath: /app/config.yaml
          subPath: config.yaml
        - name: tls-certs
          mountPath: /etc/ssl/certs
          readOnly: true
      volumes:
      - name: config
        configMap:
          name: magictunnel-config
      - name: tls-certs
        secret:
          secretName: magictunnel-tls-secret
```

## Certificate Management

### Let's Encrypt with Automatic Renewal
```bash
# Install certbot
sudo apt-get install certbot

# Generate certificate
sudo certbot certonly --standalone -d magictunnel.example.com

# Set up automatic renewal
sudo crontab -e
# Add: 0 12 * * * /usr/bin/certbot renew --quiet --post-hook "systemctl reload magictunnel"
```

### Corporate CA Integration
```yaml
server:
  tls:
    mode: "application"
    cert_file: "/etc/ssl/corporate/server.crt"
    key_file: "/etc/ssl/corporate/server.key"
    ca_file: "/etc/ssl/corporate/ca-bundle.crt"
    # Validate client certificates
    client_ca_file: "/etc/ssl/corporate/client-ca.crt"
    require_client_cert: true
```

### Certificate Rotation
```bash
# Graceful certificate reload (future feature)
curl -X POST https://magictunnel.example.com/admin/reload-certificates \
  -H "Authorization: Bearer $ADMIN_TOKEN"

# Or restart service
sudo systemctl reload magictunnel
```

## Troubleshooting

### Common Issues

#### 1. Certificate Validation Errors
```bash
# Check certificate validity
openssl x509 -in cert.pem -text -noout

# Verify certificate chain
openssl verify -CAfile ca.pem cert.pem

# Test TLS connection
openssl s_client -connect magictunnel.example.com:3000
```

#### 2. WebSocket Connection Issues
```bash
# Test WebSocket over TLS
wscat -c wss://magictunnel.example.com/mcp/ws

# Check proxy WebSocket configuration
curl -H "Upgrade: websocket" -H "Connection: upgrade" \
  https://magictunnel.example.com/mcp/ws
```

#### 3. Header Forwarding Issues
```bash
# Check if headers are properly forwarded
curl -H "X-Test-Header: value" https://magictunnel.example.com/health -v

# Verify client IP detection
curl https://magictunnel.example.com/debug/headers
```

### Monitoring TLS Health
```bash
# Check certificate expiration
curl -s https://magictunnel.example.com | openssl x509 -noout -dates

# Monitor TLS metrics (future feature)
curl https://magictunnel.example.com/metrics | grep tls_

# Health check with TLS
curl https://magictunnel.example.com/health
```

## Security Best Practices

1. **Use Strong Certificates**: Minimum 2048-bit RSA or 256-bit ECDSA
2. **Enable HSTS**: Force HTTPS connections
3. **Regular Updates**: Keep TLS libraries updated
4. **Monitor Expiration**: Set up certificate expiration alerts
5. **Secure Storage**: Protect private keys with proper file permissions
6. **Network Security**: Use secure networks for internal communication
7. **Regular Audits**: Perform regular security audits and penetration testing

## Performance Considerations

- **TLS Overhead**: ~5-10% performance impact with application-level TLS
- **Connection Reuse**: Enable HTTP/2 and connection pooling
- **Certificate Caching**: Cache certificate validation results
- **Hardware Acceleration**: Use hardware TLS acceleration when available

This guide covers the most common TLS deployment scenarios. For specific environments or advanced configurations, refer to the TLS Architecture document.
