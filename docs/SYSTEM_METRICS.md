# MagicTunnel System Metrics

This document describes the enhanced system metrics monitoring implementation in MagicTunnel v0.3.9.

## Overview

MagicTunnel provides comprehensive system monitoring that displays both system-wide resource usage and process-specific metrics for MagicTunnel services. This enables real-time operational visibility for administrators.

## Features

### System-Wide Metrics
- **CPU Usage**: Total system CPU percentage utilization
- **Memory Usage**: Current system memory consumption (MB)
- **Memory Total**: Automatically detected total system memory (e.g., 32GB)
- **Disk Usage**: System-wide disk utilization percentage
- **MCP Connections**: Active MCP client connections

### Process-Specific Metrics
- **MagicTunnel Process**
  - CPU usage percentage
  - Memory consumption (MB)  
  - Process status (running/stopped)
- **Supervisor Process**
  - CPU usage percentage
  - Memory consumption (MB)
  - Process status (running/stopped)

## Implementation Details

### Backend API

#### Endpoint
```
GET /dashboard/api/metrics
```

#### Response Format
```json
{
  "timestamp": "2025-01-21T12:00:00Z",
  "uptime_seconds": 86400,
  "system": {
    "cpu_usage_percent": 25.4,
    "memory_usage_mb": 8192,
    "memory_total_mb": 32768,
    "disk_usage_percent": 45.2
  },
  "processes": {
    "magictunnel": {
      "cpu_usage_percent": 2.1,
      "memory_usage_mb": 128,
      "status": "running"
    },
    "supervisor": {
      "cpu_usage_percent": 0.5,
      "memory_usage_mb": 64,
      "status": "running"
    }
  },
  "mcp_services": {
    "total_requests": 1234,
    "total_errors": 5,
    "avg_response_time_ms": 150.2,
    "connections": {
      "active": 3,
      "details": {...}
    }
  }
}
```

### Frontend Implementation

#### Components Updated

1. **TopBar Status Dropdown** (`/frontend/src/lib/components/layout/TopBar.svelte`)
   - Shows system totals (CPU, memory, connections)
   - Displays process-specific metrics in expandable section
   - Real-time updates every 30 seconds

2. **SystemMetricsCard** (`/frontend/src/lib/components/SystemMetricsCard.svelte`)
   - Dashboard widget showing comprehensive metrics
   - Process status indicators
   - Progress bars for resource utilization

3. **Shared System Metrics Store** (`/frontend/src/lib/stores/systemMetrics.ts`)
   - Centralized data fetching and caching
   - Synchronized updates across all components
   - Automatic subscription management

#### Data Synchronization

The implementation uses a shared store pattern to ensure all UI components display consistent data:

```typescript
// Shared service ensures synchronized fetching
systemMetricsService.start(); // Start when components mount
systemMetricsService.stop();  // Stop when components unmount

// All components use the same reactive store
$: if ($systemMetrics) {
  // Update component state from shared store
}
```

## System Detection

### Memory Detection
- **macOS**: Uses `sysctl -n hw.memsize` to detect total system memory
- **Linux**: Parses `/proc/meminfo` for MemTotal
- **Fallback**: Returns 0 for unknown systems

### Process Detection
- **Command**: Uses `ps aux` to gather process information
- **Filtering**: Identifies processes by command name patterns
  - `magictunnel` (excludes supervisor and node processes)
  - `magictunnel-supervisor`
- **Metrics Extracted**:
  - CPU percentage from ps output
  - Memory percentage converted to MB using total system memory

### Disk Usage Detection
- **Command**: Uses `df` command to get filesystem usage
- **Calculation**: Percentage of used space on root filesystem
- **Fallback**: Returns 0.0 if detection fails

## Configuration

No additional configuration is required. The system metrics are automatically enabled and collected when MagicTunnel starts.

### Refresh Rate
- **Default**: 30 seconds
- **Location**: Defined in `systemMetricsService` 
- **Configurable**: Can be modified in the store implementation

## Monitoring Dashboard

### TopBar Status Dropdown
```
System Status
├── System Metrics
│   ├── CPU Usage: 25.4%
│   ├── Memory: 8192 MB / 32768 MB
│   └── Connections: 3 active
└── Service Processes
    ├── MagicTunnel: 2.1% CPU, 128 MB
    └── Supervisor: 0.5% CPU, 64 MB
```

### Dashboard Widget
The SystemMetricsCard provides a comprehensive view including:
- System resource utilization with progress bars
- Process status and resource consumption
- MCP service statistics
- Real-time updates with timestamps

## API Integration

### TypeScript Interfaces

```typescript
export interface ProcessMetrics {
  cpu_usage_percent: number;
  memory_usage_mb: number;
  status: string;
}

export interface SystemMetrics {
  timestamp: string;
  uptime_seconds: number;
  system: {
    cpu_usage_percent: number;
    memory_usage_mb: number;
    memory_total_mb: number;
    disk_usage_percent: number;
  };
  processes: {
    magictunnel: ProcessMetrics;
    supervisor: ProcessMetrics;
  };
  // ... other fields
}
```

### Error Handling
- **API Failures**: Graceful degradation with error state display
- **Process Not Found**: Shows 0 values and "unknown" status
- **System Detection Failure**: Falls back to default values
- **Network Issues**: Retry logic with exponential backoff

## Platform Support

### Supported Platforms
- **macOS**: Full support with native system calls
- **Linux**: Full support with `/proc` and standard commands
- **Other**: Basic support with fallback values

### Commands Used
- `ps aux` - Process information gathering
- `sysctl -n hw.memsize` (macOS) - Total memory detection
- `/proc/meminfo` (Linux) - Memory information
- `df` - Disk usage statistics

## Future Enhancements

### Planned Features
- **Historical Data**: Store and display metrics history
- **Alerting**: Configurable thresholds with notifications
- **Additional Processes**: Monitor external MCP servers
- **Custom Metrics**: Plugin system for additional monitoring
- **Export**: Metrics export for external monitoring systems

### Performance Considerations
- **Caching**: Metrics cached between updates to reduce system load
- **Batching**: Single API call fetches all metrics
- **Lazy Loading**: Components only subscribe when visible
- **Memory Management**: Automatic cleanup of metric subscriptions

## Troubleshooting

### Common Issues

1. **Process Metrics Show 0**
   - Check if MagicTunnel processes are running
   - Verify process name filtering logic
   - Ensure `ps` command has proper permissions

2. **Memory Total Shows 0**
   - System memory detection may have failed
   - Check platform-specific detection methods
   - Verify system commands are available

3. **Metrics Not Updating**
   - Check network connectivity to backend API
   - Verify shared store subscription is active
   - Check browser console for JavaScript errors

### Debug Information

Enable debug logging to troubleshoot metrics issues:
```bash
RUST_LOG=magictunnel::web::dashboard=debug ./target/release/magictunnel
```

## Implementation Files

### Backend
- `src/web/dashboard.rs` - Metrics API implementation
  - `get_system_metrics()` - Main metrics endpoint
  - `get_memory_info()` - System memory detection
  - `get_process_metrics()` - Process-specific monitoring

### Frontend
- `frontend/src/lib/api/system.ts` - API client and TypeScript interfaces
- `frontend/src/lib/stores/systemMetrics.ts` - Shared metrics store
- `frontend/src/lib/components/layout/TopBar.svelte` - Status dropdown
- `frontend/src/lib/components/SystemMetricsCard.svelte` - Dashboard widget

### Key Features of Implementation
- **Real-time Updates**: 30-second refresh interval
- **Synchronized Display**: Shared store prevents data inconsistencies
- **Graceful Degradation**: Handles failures without breaking UI
- **Platform Compatibility**: Works across different operating systems
- **Enterprise Ready**: Professional UI with comprehensive monitoring

This enhanced system metrics implementation provides the operational visibility needed for production MagicTunnel deployments.