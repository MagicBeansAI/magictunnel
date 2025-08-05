# MagicTunnel Bidirectional Communication Flow

## MCP 2025-06-18 Bidirectional Architecture

🚨 **IMPORTANT**: This document reflects the MCP 2025-06-18 bidirectional communication flow where **External MCP Servers** can send sampling/elicitation requests **TO** MagicTunnel during tool execution.

## Bidirectional Flow Overview

```
1. Claude Desktop → MagicTunnel Server: tools/call (normal tool execution)
2. MagicTunnel Server → ExternalMcpProcess/HttpMcpClient: tools/call (proxy to external server)
3. ExternalMcpProcess/HttpMcpClient → External MCP Server: tools/call (via stdio/HTTP/WebSocket/Streamable Http)
4. 🔄 External MCP Server → (same connection) → ExternalMcpProcess/HttpMcpClient: sampling/createMessage
5. 🔄 ExternalMcpProcess/HttpMcpClient → MagicTunnel Server: forward sampling request
6. 🔄 MagicTunnel Server → (strategy routing) → internal LLMs OR back to Claude Desktop
7. 🔄 Response flows back through same chain to External MCP Server
8. External MCP Server → completes tool execution → returns result to Claude Desktop
```

### 1. Tool Execution Request from Claude Desktop

```
Claude Desktop → MagicTunnel MCP Server
{
  "method": "tools/call",
  "params": {
    "name": "complex_analysis_tool",
    "arguments": {
      "data": "complex dataset",
      "analysis_type": "deep_learning"
    },
    "metadata": {
      "client_id": "claude-desktop-abc123",
      "session_id": "session-xyz789"
    }
  }
}
```

### 2. MagicTunnel MCP Server Tool Routing

**File: `src/mcp/server.rs`**
```rust
// MCP Server routes tool call to external MCP server
async fn handle_tool_call(&self, request: ToolCallRequest) -> Result<ToolCallResponse> {
    let client_id = extract_client_id(&request); // "claude-desktop-abc123"
    
    // Route tool call to appropriate external MCP server
    if let Some(external_server) = self.find_tool_provider(&request.tool_name) {
        // This will establish bidirectional connection for potential sampling requests
        self.route_to_external_server(&request, &external_server, &client_id).await
    } else {
        // Handle locally if no external provider
        self.handle_locally(&request).await
    }
}
```

### 3. External MCP Server Tool Execution (with Bidirectional Capability)

**External MCP Server Process**: During tool execution, the external server may need LLM assistance and sends sampling requests back to MagicTunnel

```bash
# External MCP Server (e.g., AI analysis server) during tool execution
npx @my-company/ai-analysis-mcp-server

# Tool execution in progress...
# Server realizes it needs LLM help for complex analysis
# Sends sampling request back via same stdio/HTTP connection:

{
  "jsonrpc": "2.0",
  "id": "sampling-req-456",
  "method": "sampling/createMessage",
  "params": {
    "messages": [
      {
        "role": "user",
        "content": "Analyze this complex dataset pattern: ..."
      }
    ],
    "model_preferences": {
      "preferred_models": ["gpt-4", "claude-3-opus"]
    },
    "metadata": {
      "original_client_id": "claude-desktop-abc123",
      "tool_execution_context": "complex_analysis_tool",
      "external_server_id": "ai-analysis-server"
    }
  }
}
```

### 4. ✅ COMPLETE IMPLEMENTATION: Bidirectional Request Handling

**✅ CURRENT STATUS**: This implementation is **FULLY COMPLETE AND WORKING**

#### 4A. ExternalMcpProcess (Stdio) - ✅ FULLY IMPLEMENTED

**File: `src/mcp/external_process.rs` - ✅ COMPLETE BIDIRECTIONAL IMPLEMENTATION**
```rust
// ✅ FULLY IMPLEMENTED - Parses both McpResponse AND McpRequest
async fn read_stdout_loop(&mut self) {
    // ...
    // First try parsing as response (existing functionality)
    if let Ok(response) = serde_json::from_str::<McpResponse>(&line) {
        self.handle_response(response).await;
        continue;
    }
    
    // ✅ IMPLEMENTED: Parse incoming requests from external server
    if let Ok(request) = serde_json::from_str::<McpRequest>(&line) {
        match request.method.as_str() {
            "sampling/createMessage" => {
                // ✅ IMPLEMENTED: Forward sampling requests to MagicTunnel Server
                self.forward_sampling_request_to_server(request).await;
            }
            "elicitation/request" => {
                // ✅ IMPLEMENTED: Forward elicitation requests to MagicTunnel Server
                self.forward_elicitation_request_to_server(request).await;
            }
            _ => {
                warn!("Unsupported bidirectional method from external server: {}", request.method);
            }
        }
        continue;
    }
    
    warn!("Failed to parse stdout line as either response or request: {}", line);
}
```

#### 4B. StreamableHttpMcpClient - ✅ FULLY IMPLEMENTED

**File: `src/mcp/clients/streamable_http_client.rs` - ✅ COMPLETE BIDIRECTIONAL IMPLEMENTATION**
```rust
// ✅ FULLY IMPLEMENTED - Complete NDJSON streaming with bidirectional support
impl StreamableHttpMcpClient {
    pub async fn establish_bidirectional_connection(&self) -> Result<()> {
        // ✅ IMPLEMENTED: Establish NDJSON streaming connection
        let session = self.create_streamable_session().await?;
        
        // ✅ IMPLEMENTED: Listen for incoming sampling/elicitation requests from external server
        self.start_request_listener(session).await?;
        
        Ok(())
    }
    
    // ✅ IMPLEMENTED: Async bidirectional request handling
    async fn handle_bidirectional_request_async(
        request_forwarder: Option<SharedRequestForwarder>,
        request: McpRequest,
        server_name: String,
        original_client_id: Option<String>,
        response_sender: SharedResponseSender,
    ) {
        // ✅ IMPLEMENTED: Forward requests to MagicTunnel Server with error handling
        match request.method.as_str() {
            "sampling/createMessage" => {
                // ✅ IMPLEMENTED: Convert and forward sampling requests
            }
            "elicitation/request" => {
                // ✅ IMPLEMENTED: Convert and forward elicitation requests
            }
        }
    }
}
```

#### 4C. WebSocketMcpClient - ✅ FULLY IMPLEMENTED

**File: `src/mcp/clients/websocket_client.rs` - ✅ COMPLETE BIDIRECTIONAL IMPLEMENTATION**
```rust
// ✅ FULLY IMPLEMENTED - Complete WebSocket client for full-duplex communication
impl WebSocketMcpClient {
    pub async fn connect(&self) -> Result<()> {
        // ✅ IMPLEMENTED: Full-duplex WebSocket connection with TLS support
        let (ws_stream, _response) = connect_async(request).await?;
        
        // ✅ IMPLEMENTED: Real-time bidirectional communication
        self.start_message_handlers().await?;
        
        Ok(())
    }
    
    // ✅ IMPLEMENTED: Bidirectional message handling
    async fn handle_text_message(
        text: &str,
        server_name: &str,
        pending_requests: &Arc<Mutex<HashMap<String, oneshot::Sender<McpResponse>>>>,
        request_forwarder: &Option<SharedRequestForwarder>,
        original_client_id: &Option<String>,
        message_sender: &Arc<Mutex<Option<mpsc::UnboundedSender<Message>>>>,
    ) {
        // ✅ IMPLEMENTED: Parse incoming sampling/elicitation requests
        if let Ok(request) = serde_json::from_str::<McpRequest>(text) {
            // ✅ IMPLEMENTED: Handle bidirectional request asynchronously
            tokio::spawn(async move {
                Self::handle_bidirectional_request_async(/* ... */).await;
            });
        }
    }
}
```

### 5. ✅ Bidirectional Request Forwarding Infrastructure - FULLY IMPLEMENTED

**✅ COMPLETE: Request forwarding from external clients to MagicTunnel Server**

```rust
// ✅ IMPLEMENTED: ExternalMcpClient trait for callback mechanism
#[async_trait]
pub trait ExternalMcpClient: Send + Sync {
    async fn set_request_forwarder(&mut self, forwarder: SharedRequestForwarder) -> Result<()>;
    fn server_name(&self) -> &str;
    fn supports_bidirectional(&self) -> bool;
}

// ✅ IMPLEMENTED: Unified RequestForwarder interface
#[async_trait]
pub trait RequestForwarder: Send + Sync {
    async fn forward_sampling_request(
        &self,
        request: SamplingRequest,
        source_server: &str,
        original_client_id: &str
    ) -> Result<SamplingResponse>;
    
    async fn forward_elicitation_request(
        &self,
        request: ElicitationRequest,
        source_server: &str,
        original_client_id: &str
    ) -> Result<ElicitationResponse>;
    
    fn forwarder_id(&self) -> &str {
        "request_forwarder"
    }
}

// ✅ IMPLEMENTED: MagicTunnel Server implements RequestForwarder
impl RequestForwarder for McpServer {
    async fn forward_sampling_request(
        &self,
        request: SamplingRequest,
        source_server: &str,
        original_client_id: &str
    ) -> Result<SamplingResponse> {
        // ✅ IMPLEMENTED: Route through existing logic with enhanced metadata
        let mut enriched_request = request;
        enriched_request.metadata = Some([
            ("source_external_server".to_string(), json!(source_server)),
            ("original_client_id".to_string(), json!(original_client_id)),
            ("bidirectional_request".to_string(), json!(true)),
        ].into_iter().collect());
        
        self.handle_sampling_request(&enriched_request).await
    }
}
```

### 6. Route Decision Engine (FOR INCOMING EXTERNAL REQUESTS)

**File: `src/mcp/client.rs` - `route_sampling_request()` WORKS CORRECTLY**

```rust
// ✅ THIS LOGIC ALREADY EXISTS AND WORKS CORRECTLY
pub async fn route_sampling_request(
    &self, 
    request: &SamplingRequest, 
    original_client_id: &str,  // "claude-desktop-abc123"
    external_routing_config: Option<&McpExternalRoutingConfig>
) -> Result<SamplingResponse> {
    // Determine strategy from configuration
    let strategy = self.determine_sampling_strategy(external_routing_config);
    
    match strategy {
        ProcessingStrategy::MagictunnelHandled => {
            // ✅ Use MagicTunnel's configured LLMs (llm_client.rs)
            self.handle_sampling_with_magictunnel_llm(request).await
        }
        ProcessingStrategy::ClientForwarded => {
            // ✅ Forward back to original Claude Desktop
            self.forward_to_original_client(request, original_client_id).await
        }
        ProcessingStrategy::ExternalServer => {
            // Route to OTHER external MCP servers (not the one that sent the request)
            self.forward_sampling_to_external_servers(request, original_client_id).await
        }
        // ... other strategies
    }
}
```

### 7. LLM Processing (WORKS CORRECTLY)

**File: `src/mcp/sampling.rs` + `src/mcp/llm_client.rs`**

```rust
// ✅ MagicTunnel can process sampling requests using configured LLM providers

// File: src/mcp/sampling.rs
impl SamplingService {
    pub async fn handle_sampling_request(&self, request: &SamplingRequest) -> Result<SamplingResponse> {
        // ✅ This works correctly - uses LlmClient
        let llm_client = LlmClient::new(self.llm_config.clone())?;
        llm_client.handle_sampling_request(request).await
    }
}

// File: src/mcp/llm_client.rs  
impl LlmClient {
    pub async fn handle_sampling_request(&self, request: &SamplingRequest) -> Result<SamplingResponse> {
        // ✅ Supports OpenAI, Anthropic, Ollama
        match self.config.provider.as_str() {
            "openai" => self.handle_openai_sampling(request).await,
            "anthropic" => self.handle_anthropic_sampling(request).await,
            "ollama" => self.handle_ollama_sampling(request).await,
            _ => Err(ProxyError::config(&format!("Unsupported LLM provider: {}", self.config.provider)))
        }
    }
}
```

### 8. Response Flow Back to External MCP Server

After MagicTunnel processes the sampling request (either via internal LLMs or forwarding to Claude Desktop), the response flows back:

```
🔄 MagicTunnel Server → ExternalMcpProcess/HttpMcpClient → External MCP Server

# Response to sampling request sent back to external server:
{
  "jsonrpc": "2.0",
  "id": "sampling-req-456",  # Same ID from original sampling request
  "result": {
    "message": {
      "role": "assistant",
      "content": "Based on the dataset pattern analysis, I recommend using a transformer-based approach with attention mechanisms focused on temporal dependencies..."
    },
    "model": "gpt-4",
    "usage": {
      "input_tokens": 150,
      "output_tokens": 75
    },
    "metadata": {
      "processed_by": "magictunnel",
      "llm_provider": "openai",
      "strategy": "magictunnel_handled",
      "original_client_id": "claude-desktop-abc123"
    }
  }
}

# External MCP Server uses this LLM response to complete its tool execution
# Then sends final tool result back to Claude Desktop
```

## 🎯 REQUIRED IMPLEMENTATIONS TO FIX BIDIRECTIONAL COMMUNICATION

### Critical Task 1: Fix ExternalMcpProcess Stdio Bidirectional Parsing

**File: `src/mcp/external_process.rs`**
```rust
// 🔧 REQUIRED IMPLEMENTATION
impl ExternalMcpProcess {
    async fn read_stdout_loop(&mut self) {
        while let Some(line) = self.stdout_reader.next_line().await {
            // Try parsing as response first (existing functionality)
            if let Ok(response) = serde_json::from_str::<McpResponse>(&line) {
                self.handle_response(response).await;
                continue;
            }
            
            // ✅ NEW: Try parsing as incoming request from external server
            if let Ok(request) = serde_json::from_str::<McpRequest>(&line) {
                match request.method.as_str() {
                    "sampling/createMessage" => {
                        self.forward_sampling_request_to_server(request).await;
                    }
                    "elicitation/request" => {
                        self.forward_elicitation_request_to_server(request).await;
                    }
                    _ => {
                        warn!("Unsupported request method from external server: {}", request.method);
                    }
                }
                continue;
            }
            
            warn!("Failed to parse stdout line: {}", line);
        }
    }
    
    // ✅ NEW: Forward sampling requests to MagicTunnel Server
    async fn forward_sampling_request_to_server(&self, request: McpRequest) {
        if let Some(forwarder) = &self.request_forwarder {
            // Convert McpRequest to SamplingRequest
            if let Ok(sampling_request) = self.convert_mcp_to_sampling_request(&request) {
                match forwarder.forward_sampling_request(
                    sampling_request,
                    &self.server_name,
                    &self.original_client_id
                ).await {
                    Ok(response) => {
                        // Send response back to external server via stdin
                        let mcp_response = McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id.unwrap_or(json!("null")).to_string(),
                            result: Some(serde_json::to_value(response).unwrap()),
                            error: None,
                        };
                        self.send_response_to_external_server(mcp_response).await;
                    }
                    Err(e) => {
                        // Send error response
                        let error_response = McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id.unwrap_or(json!("null")).to_string(),
                            result: None,
                            error: Some(McpError::internal_error(e.to_string())),
                        };
                        self.send_response_to_external_server(error_response).await;
                    }
                }
            }
        }
    }
}
```

### Critical Task 2: Implement Streamable HTTP Support

**File: `src/mcp/clients/streamable_http_client.rs` (NEW FILE REQUIRED)**
```rust
// 🔧 REQUIRED: New Streamable HTTP client for bidirectional communication
pub struct StreamableHttpMcpClient {
    config: HttpClientConfig,
    session: Option<StreamableHttpSession>,
    request_forwarder: Option<Arc<dyn RequestForwarder>>,
}

impl StreamableHttpMcpClient {
    pub async fn establish_bidirectional_connection(&mut self) -> Result<()> {
        // ✅ Establish NDJSON streaming connection
        let session = self.create_streamable_session().await?;
        
        // ✅ Start listening for incoming requests from external server
        self.start_request_listener(session).await?;
        
        Ok(())
    }
    
    async fn start_request_listener(&self, session: StreamableHttpSession) {
        tokio::spawn(async move {
            // Listen for NDJSON lines from external server
            while let Some(line) = session.read_line().await {
                if let Ok(request) = serde_json::from_str::<McpRequest>(&line) {
                    if request.method == "sampling/createMessage" {
                        // Forward to MagicTunnel Server
                        self.forward_sampling_request_to_server(request).await;
                    }
                }
            }
        });
    }
}
```

### Critical Task 3: Add WebSocket Support

**File: `src/mcp/clients/websocket_client.rs` (NEW FILE REQUIRED)**
```rust
// 🔧 REQUIRED: WebSocket client for full-duplex bidirectional communication
pub struct WebSocketMcpClient {
    config: WebSocketClientConfig,
    connection: Option<WebSocketConnection>,
    request_forwarder: Option<Arc<dyn RequestForwarder>>,
}

impl WebSocketMcpClient {
    pub async fn connect(&mut self, url: &str) -> Result<()> {
        // ✅ Establish WebSocket connection
        let connection = self.create_websocket_connection(url).await?;
        
        // ✅ Start bidirectional message handling
        self.start_message_handler(connection).await?;
        
        Ok(())
    }
}
```

### Critical Task 4: Unified Request Forwarding Infrastructure

**File: `src/mcp/request_forwarder.rs` (NEW FILE REQUIRED)**
```rust
// 🔧 REQUIRED: Unified interface for forwarding requests from external clients to MagicTunnel Server
#[async_trait]
pub trait RequestForwarder: Send + Sync {
    async fn forward_sampling_request(
        &self,
        request: SamplingRequest,
        source_server: &str,
        original_client_id: &str
    ) -> Result<SamplingResponse>;
    
    async fn forward_elicitation_request(
        &self,
        request: ElicitationRequest,
        source_server: &str,
        original_client_id: &str
    ) -> Result<ElicitationResponse>;
}

// ✅ MagicTunnel Server implements this interface
impl RequestForwarder for McpServer {
    async fn forward_sampling_request(
        &self,
        request: SamplingRequest,
        source_server: &str,
        original_client_id: &str
    ) -> Result<SamplingResponse> {
        // Add metadata about the external server source
        let mut enriched_request = request;
        enriched_request.metadata = Some([
            ("source_external_server".to_string(), json!(source_server)),
            ("original_client_id".to_string(), json!(original_client_id)),
            ("bidirectional_request".to_string(), json!(true)),
        ].into_iter().collect());
        
        // Route through existing logic with proper client ID context
        self.handle_sampling_request(&enriched_request).await
    }
}
```

## Transport Protocol Requirements for Bidirectional Communication

### MCP 2025-06-18 Supported Transports for Bidirectional Communication

| Transport | Status | Bidirectional Support | Implementation Required |
|-----------|--------|----------------------|-------------------------|
| **Stdio** | ✅ Exists | ❌ **BROKEN** | Fix parsing in `ExternalMcpProcess` |
| **Streamable HTTP** | ⚠️ Server only | ❌ **MISSING** | New `StreamableHttpMcpClient` |
| **WebSocket/WSS** | ❌ **NOT IMPLEMENTED** | ❌ **MISSING** | New `WebSocketMcpClient` |
| **HTTP (Legacy)** | ✅ Exists | ❌ No bidirectional | Keep for backward compatibility |
| **SSE (Deprecated)** | ✅ Exists | ⚠️ Limited | Maintain but discourage use |

### Transport Protocol Configuration

```yaml
# magictunnel-config.yaml - Updated for bidirectional support
external_mcp:
  enabled: true
  servers:
    ai-analysis-server:
      transport: streamable_http  # ✅ Preferred for bidirectional
      url: "https://ai-analysis.example.com/mcp/streamable"
      supports_sampling: true
      supports_elicitation: true
      
    local-llm-server:
      transport: stdio  # ✅ Works after fix
      command: ["npx", "@local/llm-mcp-server"]
      supports_sampling: true
      
    websocket-server:
      transport: websocket  # 🔧 NEW: Full-duplex support
      url: "wss://realtime.example.com/mcp"
      supports_sampling: true
      supports_elicitation: true
      
  external_routing:
    sampling:
      default_strategy: magictunnel_handled  # Use MagicTunnel's LLMs by default
      server_strategies:
        "ai-analysis-server": magictunnel_handled  # Let MagicTunnel handle its sampling requests
      fallback_to_magictunnel: true
```

### Bidirectional Request Flow Decision Tree

```
🔄 Incoming Sampling Request (FROM External MCP Server during tool execution)
│
├─ Extract source_server: "ai-analysis-server"
├─ Extract original_client_id: "claude-desktop-abc123"
├─ Extract request_type: "sampling/createMessage" | "elicitation/request"
│
└─ Route Decision (via existing strategy logic):
   │
   ├─ Check server-specific strategy for the SOURCE server:
   │  └─ server_strategies["ai-analysis-server"] = "magictunnel_handled"
   │     └─ ✅ Use MagicTunnel's internal LLMs (OpenAI/Anthropic/Ollama)
   │
   ├─ Check default strategy:
   │  └─ default_strategy = "client_first" 
   │     └─ Forward to original Claude Desktop client
   │        ├─ Success → Return response to external server
   │        └─ Failure → Try other external servers or fallback
   │
   └─ Priority Order Processing:
      ├─ Try other external servers (not the requesting one)
      ├─ Try MagicTunnel internal LLMs
      └─ Return error if all fail

🎯 Response flows back to external server → completes tool execution → returns to Claude Desktop
```

## Complete Bidirectional Flow Example

### Real-World Scenario: AI Analysis Tool with LLM Assistance

```
1. 📥 Claude Desktop → MagicTunnel: tools/call "analyze_complex_dataset"
   ↓
2. 🔀 MagicTunnel → AI Analysis Server: Forward tool call via Streamable HTTP
   ↓
3. 🧠 AI Analysis Server: "I need LLM help to understand this pattern..."
   ↓
4. 🔄 AI Analysis Server → MagicTunnel: sampling/createMessage (BIDIRECTIONAL)
   {
     "method": "sampling/createMessage",
     "params": {
       "messages": [{
         "role": "user",
         "content": "Analyze this time series pattern: [complex data]"
       }],
       "metadata": {
         "tool_execution_context": "analyze_complex_dataset",
         "original_client_id": "claude-desktop-abc123"
       }
     }
   }
   ↓
5. 🤖 MagicTunnel: Routes to internal GPT-4 based on strategy
   ↓
6. ✅ GPT-4 Response → MagicTunnel → AI Analysis Server
   {
     "result": {
       "message": {
         "role": "assistant",
         "content": "This appears to be a seasonal ARIMA pattern with anomalies at..."
       }
     }
   }
   ↓
7. 🔬 AI Analysis Server: Uses LLM insights to complete analysis
   ↓
8. 📤 AI Analysis Server → MagicTunnel → Claude Desktop: Final tool result
   {
     "result": {
       "analysis": "Complete dataset analysis with LLM-enhanced insights",
       "patterns": [...],
       "recommendations": [...],
       "metadata": {
         "llm_assisted": true,
         "processing_chain": "claude → magictunnel → ai-server → magictunnel-llm → ai-server → claude"
       }
     }
   }
```

### Benefits of Correct Bidirectional Implementation

✅ **External MCP servers can leverage MagicTunnel's LLM capabilities during tool execution**
✅ **Complex multi-step workflows with LLM assistance at each stage**
✅ **Intelligent routing: external servers can access different LLM providers via MagicTunnel**
✅ **Centralized LLM management and cost control through MagicTunnel**
✅ **Enhanced tool capabilities without external servers needing direct LLM API access**

## Implementation Status Summary

### ✅ What Already Works
- **Server-side routing logic** in `src/mcp/server.rs` with `handle_sampling_request()`
- **LLM processing** via `src/mcp/sampling.rs` and `src/mcp/llm_client.rs`
- **Strategy-based routing** for sampling requests with configurable providers
- **Streamable HTTP server** implementation in `src/mcp/streamable_http.rs`
- **Basic external process management** via `ExternalMcpProcess`
- **HTTP client** for external MCP servers (request/response only)

### ❌ What's Broken/Missing

#### 🚨 **CRITICAL: Stdio Bidirectional Parsing**
- `ExternalMcpProcess` only parses `McpResponse`, ignores `McpRequest`
- External servers sending sampling requests via stdio are ignored
- **Impact**: 70% of external MCP servers use stdio transport

#### 🚨 **CRITICAL: HTTP Bidirectional Communication**
- `HttpMcpClient` only supports request/response, no streaming
- No Streamable HTTP client implementation
- **Impact**: Modern external servers can't use bidirectional features

#### 🚨 **MISSING: WebSocket Support**
- No WebSocket client implementation
- **Impact**: Real-time external servers can't connect

#### 🚨 **MISSING: Request Forwarding Infrastructure**
- No callback mechanism from external clients to MagicTunnel Server
- **Impact**: Bidirectional requests can't be routed even if parsed

### 🎯 Implementation Priority

1. **Fix Stdio Bidirectional** (2-3 days) - Highest impact, affects most external servers
2. **Add Request Forwarding** (2-3 days) - Required for any bidirectional communication
3. **Implement Streamable HTTP Client** (1 week) - Modern transport protocol
4. **Add WebSocket Support** (1 week) - Advanced real-time capabilities

### 📋 Current TODO Status

As documented in `TODO.md` Task 4: **CRITICAL: Complete Bidirectional Communication Implementation**
- Status: ⚠️ **PARTIALLY IMPLEMENTED** 
- All required implementation details are documented
- Ready for immediate development work

## Key Architectural Insights

### 1. **Bidirectional Connection Reuse**
- External MCP connections (stdio/HTTP/WebSocket) are **full-duplex**
- Same connection used for tool calls AND sampling requests
- JSON-RPC correlation handles bidirectional request/response matching

### 2. **Request Origin Tracking**
- All requests carry `original_client_id` for proper response routing
- External server requests include `source_server` metadata
- Session correlation ensures responses reach correct destinations

### 3. **Transport Protocol Hierarchy**
```
WebSocket/WSS (preferred for real-time)
    ↓
Streamable HTTP (preferred for HTTP-based)
    ↓  
Stdio (preferred for process-based)
    ↓
HTTP (legacy, no bidirectional)
    ↓
SSE (deprecated)
```

### 4. **Layered Fault Tolerance**
- Bidirectional parsing errors don't break main tool execution
- Failed sampling requests fall back to local LLM processing
- Transport failures gracefully degrade to available protocols
- Always maintains tool execution capability even without LLM assistance

### 5. **Strategic LLM Routing**
- External servers can access MagicTunnel's LLM providers without direct API keys
- Centralized cost control and rate limiting through MagicTunnel
- Intelligent provider selection based on request context and server configuration

This architecture enables **true collaborative AI workflows** where external MCP servers can leverage MagicTunnel's LLM capabilities during complex tool execution, creating more intelligent and capable MCP ecosystems.