# 🖥️ Web Dashboard Guide

The MagicTunnel Web Dashboard provides a comprehensive interface for managing, monitoring, and testing your MCP proxy server.

## Quick Access

### **Recommended Quick Start (Full Setup)**
```bash
# One-line setup with smart discovery (Ollama + development mode)
make build-release-semantic && make pregenerate-embeddings-ollama MAGICTUNNEL_ENV=development

# Start MagicTunnel with supervisor (includes web dashboard)
./target/release/magictunnel-supervisor

# Access the dashboard
open http://localhost:5173/dashboard
```

### **Quick Start (Pre-built Binary)**
```bash
# If already built, just start the supervisor
./target/release/magictunnel-supervisor

# Access the dashboard
open http://localhost:5173/dashboard
```

## Architecture Overview

The web dashboard operates in a multi-tier architecture with service container integration (v0.3.11):

```
┌─────────────────┐    ┌─────────────────────────────────────┐    ┌─────────────────┐
│ Frontend (5173) │───►│        MagicTunnel API              │───►│ Supervisor TCP  │
│ Svelte + Vite   │    │           (3001)                    │    │ (8081)          │
│                 │    │                                     │    │                 │
│ ┌─────────────┐ │    │  ┌─────────────────────────────────┐ │    │ ┌─────────────┐ │
│ │Banner Store │ │    │  │      Service Container         │ │    │ │Process Mgmt │ │
│ │  System     │ │    │  │                                 │ │    │ │             │ │
│ └─────────────┘ │    │  │ Proxy Mode:                     │ │    │ └─────────────┘ │
└─────────────────┘    │  │ ├─ ProxyServices                │ │    └─────────────────┘
        │              │  │ └─ AdvancedServices: None       │ │            │
        │              │  │                                 │ │            ▼
        │              │  │ Advanced Mode:                  │ │  ┌─────────────────┐
        │              │  │ ├─ ProxyServices (foundation)   │ │  │ Environment Var │
        │              │  │ └─ AdvancedServices (security)  │ │  │ Management      │
        │              │  └─────────────────────────────────┘ │  └─────────────────┘
        │              └─────────────────────────────────────┘
        │                        │                 │
        │                        ▼                 ▼
        │              ┌─────────────────┐ ┌─────────────────┐
        │              │ External MCP    │ │ Mode Detection  │
        │              │ Services (8082+)│ │ & Status API    │
        │              └─────────────────┘ └─────────────────┘
        ▼
API Proxy: /dashboard/api/* → localhost:3001
Banner Integration: Real-time status updates
```

## Dashboard Features

### 📊 System Overview
- **Real-time Status**: Live system health monitoring
- **Performance Metrics**: CPU, memory, and uptime tracking
- **Service Status**: Quick overview of all running services
- **Auto-refresh**: 30-second intervals with countdown timer

### 🔧 Tool Management
- **Tool Catalog**: Browse all available MCP tools with search and filtering
- **Tool Testing**: Interactive parameter forms for testing any tool
- **Schema Validation**: Dynamic form generation based on tool schemas
- **Tool Status**: Enable/disable and show/hide tools
- **Execution History**: View tool execution results and performance

### 📋 Configuration Management
- **YAML Editor**: Full-featured configuration editor with syntax highlighting
- **Real-time Validation**: Instant feedback on configuration errors
- **Backup & Restore**: Automatic backups with one-click restore
- **Template Loading**: Load configuration templates and examples
- **Hot Reload**: Apply configuration changes without restart

### 📝 Live Log Viewer
- **Real-time Streaming**: Live log updates with color-coded levels
- **Advanced Filtering**: Filter by log level, source, and search terms
- **Export Functionality**: Download logs as CSV for analysis
- **Pagination**: Efficient handling of large log files
- **Search & Highlight**: Find specific log entries quickly

### 🔍 MCP Testing Interface
- **JSON-RPC 2.0**: Full MCP protocol testing with autocomplete
- **Method Support**: Test all MCP methods (tools/list, tools/call, etc.)
- **Parameter Validation**: Real-time JSON validation and syntax highlighting
- **Subprocess Management**: Direct MCP communication via stdio
- **Response Analysis**: Detailed response inspection and error handling

### 🌐 Smart Discovery Visualizer
- **Natural Language Input**: Test smart tool discovery with plain English
- **Confidence Visualization**: Color-coded confidence scores and progress bars
- **Alternative Suggestions**: Multiple tool options with reasoning
- **Parameter Mapping**: Automatic parameter extraction and preview
- **Dual Protocol**: Toggle between HTTP API and MCP protocol execution

### ⚙️ Service Management
- **External MCP Services**: Monitor and manage all external MCP servers
- **Health Monitoring**: Real-time service health and connectivity status
- **Process Control**: Start, stop, and restart services from the web interface
- **Service Configuration**: View service settings and environment variables

### 🔄 Environment & System Control
- **Environment Variables**: View and manage system environment variables
- **API Key Management**: Secure handling of API keys with masking
- **System Restart**: Graceful system restart with custom workflows
- **Supervisor Integration**: Advanced process management and monitoring
- **Unified Status Banner**: Real-time status updates with modern minimal design
- **Mode Switching**: Seamless runtime mode switching with status feedback

### 🎨 Unified Status Banner System (v0.3.11)

The dashboard features a modern, unified status banner system that replaces the traditional bulky alert banners:

**Features:**
- **Dynamic Status Updates**: Real-time feedback for restart/mode switch operations
- **Color-Coded Types**: Success (green), error (red), warning (orange), info (blue)
- **Space Efficient**: 60% smaller height with consistent horizontal layout
- **Auto-Clear**: Success messages automatically clear after 5 seconds
- **Mode Aware**: Shows current runtime mode when no operations are active

**Status Examples:**
```
[●] Running in Proxy Mode • Core features only
[●] Restarting System (15s remaining) • System restarting...
[●] Mode Switch Complete • Successfully switched to advanced mode
[●] Error occurred • Check system logs for details
```

**Technical Implementation:**
- **Global Store**: `/frontend/src/lib/stores/banner.ts` manages banner state
- **Component Integration**: `ModeAwareLayout.svelte` displays banner with modern CSS
- **API Integration**: Seamless integration with supervisor restart/status APIs

## Development Setup

### Frontend Development
```bash
# Navigate to frontend directory
cd frontend

# Install dependencies
npm install

# Start development server (hot reload)
npm run dev

# Build for production
npm run build

# Type checking
npm run check
```

### Backend Integration
```bash
# Run backend with hot reload
cargo watch -x run

# Build with embedded frontend
cargo build --release

# Run tests
cargo test
```

## Port Configuration

| Service | Port | Purpose |
|---------|------|---------|
| **Frontend UI** | 5173 | Vite development server with hot reload |
| **MagicTunnel API** | 3001 | Backend API server (configurable) |
| **Supervisor TCP** | 8081 | Process management and control |
| **External MCP** | 8082+ | External MCP service ports |

## API Endpoints

The dashboard communicates with the backend via REST API:

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/dashboard/api/status` | GET | System health and metrics |
| `/dashboard/api/tools` | GET | Available tools catalog |
| `/dashboard/api/tools/{name}/execute` | POST | Execute specific tool |
| `/dashboard/api/services` | GET | External MCP services status |
| `/dashboard/api/config` | GET/POST | Configuration management |
| `/dashboard/api/logs` | GET | Log entries with filtering |
| `/dashboard/api/mcp/execute` | POST | MCP command execution |
| `/dashboard/api/system/restart` | POST | System restart control |

## Security Features

- **Process Isolation**: Frontend and backend run in separate processes
- **API Validation**: All API requests are validated and sanitized
- **Environment Protection**: Sensitive environment variables are masked
- **Secure Defaults**: Safe configuration defaults and validation
- **Access Control**: Dashboard access can be protected via authentication

## Troubleshooting

### Dashboard Not Loading
1. **Check Services**: Ensure supervisor is running on port 8081
2. **Verify Backend**: MagicTunnel API should be running on port 3001
3. **Frontend Issues**: Check if Vite dev server is running on port 5173
4. **Port Conflicts**: Ensure no other services are using required ports

### API Connection Issues
1. **Proxy Configuration**: Verify Vite proxy settings in `vite.config.ts`
2. **CORS Issues**: Check if backend allows dashboard origin
3. **Network Connectivity**: Test API endpoints directly with curl
4. **Firewall**: Ensure local firewall allows connection between services

### Performance Issues
1. **Auto-refresh**: Disable auto-refresh if experiencing high load
2. **Log Pagination**: Reduce log page size for better performance
3. **Tool Filtering**: Use search and filters to reduce rendered content
4. **Browser Cache**: Clear browser cache if seeing stale content

## Advanced Features

### Configuration Backup System
The dashboard automatically creates timestamped backups of all configuration changes, allowing you to:
- View backup history with timestamps
- Restore previous configurations
- Compare configuration versions
- Export/import configuration files

### Smart Discovery Integration
Advanced integration with MagicTunnel's Smart Discovery system:
- **Visual Confidence Scoring**: See how confident the system is in its tool selection
- **Alternative Options**: View other potential tool matches
- **Parameter Assistance**: Automatic parameter extraction and mapping
- **Interactive Results**: Execute discovered tools directly from results

### Real-time Monitoring
- **Live Updates**: Real-time system status without page refresh
- **Performance Tracking**: Monitor response times and system load
- **Alert System**: Visual indicators for system issues
- **Uptime Tracking**: Continuous uptime monitoring and statistics

The web dashboard transforms MagicTunnel from a command-line tool into a user-friendly platform suitable for both technical and non-technical users.