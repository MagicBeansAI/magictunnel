# Monitoring and Observability Dashboard Endpoints

The following endpoints have been added to the MagicTunnel web dashboard to provide comprehensive monitoring and observability features:

## New API Endpoints

### 1. System Metrics
**GET `/dashboard/api/metrics`**

Returns comprehensive system metrics including:
- System uptime and performance data
- MCP service statistics  
- Tool execution metrics
- External service health status

```json
{
  "timestamp": "2025-01-30T12:00:00Z",
  "uptime_seconds": 86400,
  "system": {
    "cpu_usage_percent": 25.5,
    "memory_usage_mb": 512.0,
    "disk_usage_percent": 45.2
  },
  "mcp_services": {
    "total_requests": 1234,
    "total_errors": 12,
    "avg_response_time_ms": 150.5,
    "external_services": {
      "total": 3,
      "health_status": {
        "filesystem_server": "healthy",
        "git_server": "healthy", 
        "sse_test_server": "healthy"
      }
    }
  },
  "tools": {
    "total_tools": 45,
    "visible_tools": 30,
    "hidden_tools": 15,
    "execution_stats": {
      "total_executions": 567,
      "successful_executions": 543,
      "failed_executions": 24,
      "avg_execution_time_ms": 89.3
    }
  }
}
```

### 2. Service Metrics  
**GET `/dashboard/api/metrics/services`**

Returns detailed metrics for each MCP service:

```json
{
  "timestamp": "2025-01-30T12:00:00Z",
  "process_services": {
    "filesystem_server": {
      "status": "healthy",
      "tools_count": 12,
      "last_updated": "2025-01-30T12:00:00Z"
    }
  },
  "network_services": {
    "sse_test_server": {
      "status": "healthy", 
      "tools_count": 4,
      "last_updated": "2025-01-30T12:00:00Z"
    }
  }
}
```

### 3. Health Status
**GET `/dashboard/api/health`**

Returns comprehensive health status of all system components:

```json
{
  "timestamp": "2025-01-30T12:00:00Z",
  "overall_status": "healthy",
  "uptime_seconds": 86400,
  "components": {
    "registry": {
      "status": "healthy",
      "tools_loaded": 45,
      "last_updated": "2025-01-30T12:00:00Z"
    },
    "mcp_server": {
      "status": "healthy",
      "active_connections": 2,
      "requests_processed": 1234
    },
    "external_services": {
      "total": 3,
      "healthy": 3,
      "unhealthy": 0,
      "unknown": 0
    }
  }
}
```

### 4. System Alerts
**GET `/dashboard/api/observability/alerts`**

Returns active system alerts and warnings:

```json
{
  "timestamp": "2025-01-30T12:00:00Z",
  "alerts": [
    {
      "id": "service_down_git_server",
      "severity": "critical",
      "title": "Service git_server Down",
      "description": "MCP service 'git_server' is not running",
      "timestamp": "2025-01-30T11:45:00Z",
      "category": "external_service"
    }
  ],
  "total_alerts": 1,
  "critical_count": 1,
  "error_count": 0,
  "warning_count": 0
}
```

## Frontend Integration

### Dashboard Sections

1. **System Overview**
   - Real-time system status indicators
   - Key performance metrics
   - Service health summary

2. **Service Monitoring**
   - Individual service status cards
   - Performance graphs
   - Connection status indicators

3. **Alerts Panel**
   - Active alerts with severity indicators
   - Alert history
   - Notification settings

4. **Metrics Dashboard**
   - Interactive charts and graphs
   - Historical trend analysis
   - Customizable time ranges

### UI Components

The monitoring features should be accessible through:

- **Navigation Menu**: Add "Monitoring" section
- **Dashboard Widgets**: Real-time status cards on main dashboard
- **Alerts Badge**: Notification indicator in header
- **Service Status**: Visual indicators in service lists

### Implementation Status

✅ **Backend API Endpoints**: Added to `src/web/dashboard.rs`
✅ **Health Status Tracking**: Integrated with external MCP manager
✅ **Metrics Collection**: Connected to MCP metrics system
✅ **Alert Generation**: Automatic alert creation based on service health

⏳ **Frontend UI**: Needs implementation in web dashboard
⏳ **Real-time Updates**: WebSocket integration for live updates
⏳ **Alert Notifications**: Browser notifications for critical alerts

## Usage Examples

### Testing the Endpoints

```bash
# Get system metrics
curl http://localhost:5173/dashboard/api/metrics

# Get service health
curl http://localhost:5173/dashboard/api/health

# Get active alerts  
curl http://localhost:5173/dashboard/api/observability/alerts

# Get detailed service metrics
curl http://localhost:5173/dashboard/api/metrics/services
```

### Integration with Existing Dashboard

The monitoring endpoints integrate with the existing dashboard features:

- **Services Tab**: Now shows real-time health status
- **Tools Tab**: Includes execution statistics
- **Configuration Tab**: Validates service health before changes
- **Logs Tab**: Correlates alerts with log entries

This provides comprehensive observability for MagicTunnel's operation, helping users monitor system health, track performance, and quickly identify issues.