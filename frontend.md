# MagicTunnel Frontend Architecture

## Overview

This document outlines the architecture and design decisions for the MagicTunnel web dashboard using Svelte. The dashboard provides a modern web interface for managing and monitoring the MagicTunnel MCP proxy system.

## Architecture Decision: Development vs Production Deployment

### Current Setup: Development Mode with Proxy
**Development Architecture (Current):**
- **Frontend UI**: Svelte dev server on port `5173` (Vite)
- **MagicTunnel API**: Backend server on port `3001` (from config) 
- **Supervisor Process**: TCP server on port `8081` (localhost only)
- **API Proxy**: Vite proxies `/dashboard/api/*` calls from 5173 → 3001

### Future Production Option: Embedded Approach
For production deployment, we can embed the Svelte frontend directly into the Rust binary using the `include_dir!` macro. This provides:

- **Single Binary Deployment**: No separate frontend server required
- **Simplified Operations**: One process to manage and deploy
- **Security Integration**: Leverages existing authentication middleware
- **Performance**: Static assets served directly from memory

### Directory Structure
```
magictunnel/
├── frontend/          # Svelte SPA
│   ├── src/
│   │   ├── lib/
│   │   │   ├── components/
│   │   │   ├── stores/
│   │   │   └── api.js
│   │   ├── routes/
│   │   └── app.html
│   ├── package.json
│   ├── svelte.config.js
│   └── dist/          # Built assets (embedded in Rust)
└── src/
    └── web/           # New Rust module
        ├── mod.rs
        ├── dashboard.rs
        └── static.rs  # Serve built Svelte assets
```

## Technology Stack

### Frontend
- **Framework**: SvelteKit (or vanilla Svelte + Vite)
- **Styling**: Tailwind CSS for rapid development and consistent design
- **State Management**: Svelte stores + Context API
- **HTTP Client**: Native fetch API
- **Build Tool**: Vite for fast development and optimized builds
- **Real-time**: WebSocket integration for live updates

### Backend Integration
- **Web Framework**: Actix-web (existing)
- **Asset Embedding**: `include_dir!` macro for static files
- **Authentication**: Existing JWT/OAuth middleware
- **API Routes**: RESTful endpoints + WebSocket for real-time data

## Core Components

### 1. Dashboard Layout
```svelte
<!-- Main layout with navigation and content area -->
+layout.svelte
├── Header (branding, user info, logout)
├── Sidebar (navigation menu)
└── Main content area (route-specific content)
```

### 2. Navigation Structure
- **Dashboard Home**: System overview with clickable status cards, health metrics, MCP testing, and build commands
- **Tools**: Tool catalog, testing interface, and execution history (accessible via clickable status card)
- **Services**: External MCP services management (accessible via clickable status card)  
- **Configuration**: System configuration viewer/editor
- **Logs**: Real-time log viewer and filtering

### 3. Real-time Data Flow
```
Rust Backend → WebSocket → Svelte Stores → Reactive Components
     ↓
  Fallback HTTP polling (2-5 second intervals)
```

## API Endpoints

### REST API
- `GET /dashboard/api/status` - System health and metrics
- `GET /dashboard/api/tools` - Available tools catalog
- `POST /dashboard/api/tools/{name}/execute` - Execute tool for testing
- `GET /dashboard/api/services` - External MCP services status
- `GET /dashboard/api/config` - System configuration (read-only)
- `GET /dashboard/api/logs` - Recent log entries with filtering
- `GET /dashboard/api/makefile` - Makefile commands with script extraction
- `POST /dashboard/api/makefile/execute` - Execute makefile commands
- `POST /dashboard/api/mcp/execute` - Execute MCP JSON-RPC 2.0 commands via stdio

### WebSocket
- `WS /dashboard/ws` - Real-time updates for:
  - System status changes
  - Tool execution results
  - Service connectivity changes
  - Live log streaming

## State Management

### Svelte Stores
```javascript
// stores/auth.js - Authentication state
// stores/tools.js - Tool catalog and execution state
// stores/services.js - External services status
// stores/websocket.js - WebSocket connection management
// stores/notifications.js - User notifications and alerts
```

### Data Flow
1. **Initial Load**: HTTP API calls populate stores
2. **Real-time Updates**: WebSocket messages update stores
3. **User Actions**: Component interactions trigger API calls
4. **Reactive UI**: Components automatically update when stores change

## Security Integration

### Authentication
- Leverages existing MagicTunnel auth middleware
- Session-based authentication for web UI
- JWT token validation for API endpoints
- Automatic logout on token expiration

### Authorization
- Role-based access control (if implemented)
- Read-only vs admin permissions
- Secure API endpoint access

## Development Workflow

### Development Mode
```bash
# Terminal 1: Start Rust backend with hot reload
cargo watch -x run

# Terminal 2: Start Svelte dev server
cd frontend && npm run dev
```

### Production Build
```bash
# Build Svelte frontend
cd frontend && npm run build

# Build Rust with embedded assets
cargo build --release
```

## Performance Considerations

### Optimization Strategies
- **Asset Compression**: Gzip/Brotli compression for static assets
- **Code Splitting**: Route-based code splitting in Svelte
- **Lazy Loading**: Components loaded on demand
- **Efficient Updates**: WebSocket debouncing and selective updates
- **Caching**: HTTP caching headers for static assets

### Bundle Size
- Target: < 100KB initial bundle
- Tree shaking for unused dependencies
- Minimal external dependencies

## Accessibility & UX

### Design Principles
- **Responsive Design**: Mobile-first approach with Tailwind CSS
- **Accessibility**: WCAG 2.1 AA compliance
- **Performance**: < 3s initial load, < 1s navigation
- **User Feedback**: Loading states, error handling, success notifications

### Component Standards
- Semantic HTML elements
- ARIA labels and roles
- Keyboard navigation support
- Screen reader compatibility

## Future Enhancements

### Planned Features
- **Dark/Light Theme**: User preference support
- **Customizable Dashboard**: Drag-and-drop widget layout
- **Advanced Filtering**: Complex search and filter capabilities
- **Export Functionality**: Data export in various formats
- **Plugin System**: Extensible component architecture

### Monitoring Integration
- **Analytics**: User interaction tracking
- **Error Reporting**: Frontend error collection
- **Performance Metrics**: Real user monitoring (RUM)

## Integration with MagicTunnel Features

### Smart Discovery Integration
- Visual tool discovery interface
- Confidence score visualization
- Parameter mapping assistance
- Discovery result history

### MCP Testing & Development Tools
- **JSON-RPC 2.0 Interface**: Native MCP protocol testing with stdio subprocess management
- **Real-time Execution**: Execute MCP commands (tools/list, tools/call, capabilities) against running MagicTunnel
- **Environment Configuration**: Uses production environment settings (API keys, Ollama, semantic search)
- **Response Visualization**: Formatted JSON responses with status indicators and error handling
- **Parameter Validation**: JSON parameter input with syntax validation and helpful examples
- **Process Management**: Automatic subprocess lifecycle with timeout protection and cleanup

### Smart Discovery Integration
- **MCP Mode Toggle**: Switch between HTTP API and MCP protocol execution seamlessly
- **Dual Protocol Support**: Execute smart tool discovery via either HTTP or MCP protocols
- **Response Structure Normalization**: Unified UI rendering for both HTTP and MCP response formats
- **Collapsible Debug Interface**: Optional debug view for inspecting response structures
- **Clean Response Processing**: Automatic deduplication of metadata and optimized data structures

### Build & Development Commands
- **Makefile Integration**: Visual display of available build and management commands
- **Script Visibility**: Expandable "Show Commands" sections revealing actual bash scripts
- **Command Execution**: Direct execution of safe makefile targets from the web interface
- **Script Extraction**: Backend parsing of Makefile targets to display underlying commands
- **Safety Validation**: Whitelist-based command execution with security checks

### Agent Router Integration
- Visual routing configuration
- Conflict resolution interface
- Performance metrics display
- Routing rule management

### External MCP Integration
- Service health monitoring
- Connection management
- Process lifecycle control
- Configuration validation

### MCP Resources and Prompts Management

#### CLI Management Tools ✅ **COMPLETE**
MagicTunnel includes comprehensive CLI management commands for MCP resources and prompts:

##### **Resources Management**
```bash
# List all available MCP resources
magictunnel-cli resources list --server http://localhost:3001

# Read resource content by URI
magictunnel-cli resources read --uri "file://path/to/resource" --server http://localhost:3001

# Export resource content to file (handles text and binary content)
magictunnel-cli resources export --uri "file://path/to/resource" --output exported_file.txt --server http://localhost:3001
```

##### **Prompts Management**
```bash
# List all prompt templates with arguments
magictunnel-cli prompts list --server http://localhost:3001

# Execute a prompt template with arguments
magictunnel-cli prompts execute "summarize_code" --args '{"language": "rust", "file": "src/main.rs"}' --server http://localhost:3001

# Export prompt execution results to JSON
magictunnel-cli prompts export "summarize_code" --args '{"language": "rust"}' --output result.json --server http://localhost:3001
```

##### **Additional Management Commands**
```bash
# Tools management
magictunnel-cli tools list                                    # List all available tools
magictunnel-cli tools execute "tool_name" --args '{}'        # Execute a tool
magictunnel-cli tools info "tool_name"                       # Get tool information

# Services management
magictunnel-cli services list                                # List MCP services with status
magictunnel-cli services restart "service_name"              # Restart a service
magictunnel-cli services start "service_name"                # Start a service
magictunnel-cli services stop "service_name"                 # Stop a service

# Server management
magictunnel-cli server status                                # Check server status
magictunnel-cli server restart                               # Restart server
magictunnel-cli server health                                # Check server health
```

#### Frontend Web Interface Integration ✅ **COMPLETE**
Web dashboard integration for resources and prompts management is now fully implemented:

- **Resources Browser Interface** ✅ - Complete visual resource management at `/resources` route
- **Prompt Templates Browser** ✅ - Interactive prompt management UI at `/prompts` route
- **Template Execution Interface** ✅ - Web-based prompt execution with form inputs and argument mapping
- **Resource Content Viewer** ✅ - In-browser resource content display with MIME type support
- **Template Argument Forms** ✅ - Dynamic forms for prompt template arguments with validation

The web interface uses the same REST API endpoints as the CLI commands, ensuring complete consistency between command-line and web-based management.

This architecture provides a solid foundation for the MagicTunnel web dashboard while maintaining the system's core principles of simplicity, performance, and reliability.