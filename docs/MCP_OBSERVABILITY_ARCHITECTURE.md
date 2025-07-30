# MCP Observability Architecture

## Overview

This document outlines the comprehensive observability system for External MCP services in MagicTunnel, providing real-time monitoring, metrics collection, alerting, and enhanced UI visibility.

## Current State Analysis

### Existing Capabilities ✅
- Basic service status (`healthy`/`unhealthy`/`unknown`)
- Process PID and uptime tracking  
- Tools count per service
- Simple health checks via capability file existence
- Basic service lifecycle management (start/stop/restart)
- Services UI with expandable details

### Critical Gaps ❌
- No real-time metrics collection (latency, throughput, errors)
- No actual MCP protocol health checks
- No historical data storage or trending
- No alerting system for service degradation
- No detailed request/response logging
- No memory/CPU usage monitoring

## Proposed Architecture

### 1. MCP Health Monitor Service

```rust
pub struct McpHealthMonitor {
    /// Metrics collection and storage
    metrics_collector: Arc<McpMetricsCollector>,
    /// Real-time health checker
    health_checker: Arc<McpHealthChecker>, 
    /// Alert manager for degradation/failures
    alert_manager: Arc<McpAlertManager>,
    /// WebSocket broadcaster for real-time UI updates
    ws_broadcaster: Arc<WebSocketBroadcaster>,
}
```

**Key Features:**
- **Real-time Health Checks**: Send actual MCP `ping` requests every 30s
- **Metrics Collection**: Track latency, throughput, error rates, success rates
- **Alert Generation**: Configurable thresholds for health degradation
- **Live Updates**: WebSocket broadcasting to dashboard UI

### 2. Metrics Collection System

#### Core Metrics Tracked
```rust
pub struct McpServiceMetrics {
    // Performance Metrics
    pub request_latency_ms: Vec<f64>,          // Request latencies
    pub requests_per_minute: u64,              // Throughput
    pub success_rate: f64,                     // Success percentage
    pub error_rate: f64,                       // Error percentage
    
    // Health Metrics  
    pub consecutive_failures: u32,             // Failed health checks
    pub last_successful_request: Option<DateTime<Utc>>,
    pub uptime_percentage: f64,                // SLA tracking
    
    // Resource Metrics
    pub memory_usage_mb: Option<f64>,          // Process memory
    pub cpu_usage_percent: Option<f64>,        // Process CPU
    
    // Request Distribution
    pub request_types: HashMap<String, u64>,   // tools/list, tools/call, etc.
    pub error_types: HashMap<String, u64>,     // Error categorization
}
```

#### Storage Strategy
```rust
pub struct McpMetricsStorage {
    // In-memory for real-time (last 24h)
    recent_metrics: Arc<RwLock<VecDeque<TimestampedMetrics>>>,
    
    // File-based for historical (configurable retention)
    historical_storage: Option<PathBuf>,
    
    // Optional external storage (Prometheus, InfluxDB)
    external_storage: Option<Box<dyn ExternalMetricsStorage>>,
}
```

### 3. Enhanced Health Checking

#### Active Health Checks
```rust
pub struct McpHealthChecker {
    /// Send actual MCP protocol health checks
    pub async fn perform_health_check(&self, process: &ExternalMcpProcess) -> HealthCheckResult {
        // 1. Send MCP ping request
        // 2. Check tools/list response time
        // 3. Validate process is responsive
        // 4. Check memory/CPU if available
    }
}

pub struct HealthCheckResult {
    pub status: HealthStatus,           // Healthy, Degraded, Unhealthy, Down
    pub response_time_ms: Option<u64>,  // Actual response time
    pub error_details: Option<String>,  // Specific error information
    pub last_checked: DateTime<Utc>,    // Timestamp
    pub check_type: HealthCheckType,    // Active, Passive, Synthetic
}
```

#### Health Status Levels
- **Healthy**: All checks passing, low latency
- **Degraded**: Some issues but functional (high latency, occasional errors)
- **Unhealthy**: Significant issues (high error rate, very high latency)
- **Down**: Not responding or crashed

### 4. Real-time Dashboard Enhancements

#### WebSocket Live Updates
```typescript
// Frontend WebSocket connection for live metrics
interface McpMetricsUpdate {
    service_name: string;
    timestamp: string;
    metrics: {
        status: 'healthy' | 'degraded' | 'unhealthy' | 'down';
        response_time_ms: number;
        requests_per_minute: number;
        success_rate: number;
        error_rate: number;
        uptime_percentage: number;
    };
}
```

#### Enhanced Services UI Components
1. **Real-time Status Cards** - Live updating service status
2. **Metrics Charts** - Response time and throughput graphs  
3. **Error Rate Indicators** - Visual error rate displays
4. **SLA Dashboard** - Uptime and performance SLA tracking
5. **Alert Notifications** - In-UI alert display system

### 5. Alerting System

#### Alert Configuration
```yaml
mcp_monitoring:
  alerts:
    enabled: true
    channels:
      - type: "log"        # Log-based alerts
      - type: "webhook"    # HTTP webhook notifications
      - type: "email"      # Email notifications (future)
    
    rules:
      - name: "service_down"
        condition: "status == 'down'"
        severity: "critical" 
        cooldown_minutes: 5
        
      - name: "high_error_rate"
        condition: "error_rate > 0.1"  # 10% error rate
        severity: "warning"
        cooldown_minutes: 10
        
      - name: "high_latency"
        condition: "avg_response_time_ms > 5000"  # 5 second average
        severity: "warning" 
        cooldown_minutes: 15
```

### 6. Implementation Plan

#### Phase 1: Core Metrics Collection ⚡ HIGH PRIORITY
- [ ] Implement `McpMetricsCollector` with basic metrics
- [ ] Add metrics tracking to `ExternalMcpProcess`
- [ ] Create in-memory metrics storage
- [ ] Add metrics endpoints to dashboard API

#### Phase 2: Enhanced Health Checking ⚡ HIGH PRIORITY  
- [ ] Implement active MCP protocol health checks
- [ ] Add health status levels (Healthy/Degraded/Unhealthy/Down)
- [ ] Integrate health checks with existing process management
- [ ] Add health check configuration options

#### Phase 3: Real-time Dashboard 🚀 HIGH PRIORITY
- [ ] Add WebSocket support for live metrics updates
- [ ] Create real-time metrics components (charts, graphs)
- [ ] Enhance services page with live data
- [ ] Add metrics visualization (response times, error rates)

#### Phase 4: Alerting System 🔔 MEDIUM PRIORITY
- [ ] Implement configurable alert rules engine
- [ ] Add log-based alerting 
- [ ] Create webhook notification system
- [ ] Add alert management UI

#### Phase 5: Advanced Features 📊 FUTURE
- [ ] Historical metrics storage and analysis
- [ ] SLA/SLO monitoring and reporting
- [ ] Integration with external monitoring (Prometheus/Grafana)
- [ ] Performance trend analysis and predictions

## Technical Implementation Details

### Metrics Collection Integration Points

1. **ExternalMcpProcess**: Add metrics collection to request/response cycles
2. **ExternalMcpManager**: Coordinate health checks and metrics aggregation
3. **Dashboard API**: Expose metrics via REST endpoints and WebSocket
4. **Frontend**: Real-time metrics display and alerting UI

### Configuration Schema
```yaml
mcp_monitoring:
  enabled: true
  health_check_interval_seconds: 30
  metrics_retention_hours: 168  # 7 days
  
  storage:
    type: "memory_and_file"  # memory_only, file_only, memory_and_file
    file_path: "./data/mcp_metrics.json"
    max_file_size_mb: 100
    
  thresholds:
    response_time_warning_ms: 2000
    response_time_critical_ms: 5000
    error_rate_warning: 0.05      # 5%
    error_rate_critical: 0.15     # 15%
    consecutive_failures_critical: 5
```

## Benefits

### For Developers
- **Proactive Issue Detection**: Identify problems before they impact users
- **Performance Insights**: Understand which MCP servers are slow/fast
- **Root Cause Analysis**: Detailed error tracking and categorization

### For Operations
- **Real-time Monitoring**: Live dashboard with service health
- **SLA Monitoring**: Track uptime and performance against targets
- **Automated Alerting**: Get notified of issues immediately

### For End Users
- **Transparent Status**: See which services are available/degraded
- **Performance Expectations**: Understand response time characteristics
- **Service Reliability**: Visibility into service stability over time

This comprehensive observability system will transform MCP monitoring from basic status checks to enterprise-grade observability with real-time insights, proactive alerting, and detailed performance analytics.