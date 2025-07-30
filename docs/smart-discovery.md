# Smart Tool Discovery System

MagicTunnel features an advanced **Smart Tool Discovery System** that provides a clean, uncluttered interface while maintaining access to all capabilities through intelligent discovery. This system reduces N tools to 1 intelligent proxy tool, solving context overflow issues in MCP clients.

## Key Innovation

**The Problem:** Traditional MCP systems expose 50+ individual tools, causing context overflow in AI systems due to message limits.

**The Solution:** Smart Discovery reduces N tools to 1 intelligent proxy tool (`smart_tool_discovery`) that:
1. Analyzes natural language requests
2. Finds the best matching tool using hybrid search strategies  
3. Maps parameters from natural language to tool schema
4. Proxies the call to the actual tool
5. Returns results with discovery metadata

## Architecture Components

### 1. Smart Discovery Service (SmartDiscoveryService)
- Main orchestration layer
- Handles tool selection and parameter mapping
- Manages caching and performance optimization

### 2. Multi-Strategy Search Engine
- **Rule-based Search**: Keyword/fuzzy matching
- **Semantic Search**: Vector similarity using embeddings
- **LLM-based Search**: AI-powered tool selection
- **Hybrid Search**: Intelligent combination of all three

### 3. LLM Parameter Mapper (LlmParameterMapper)
- Extracts parameters from natural language
- Maps to tool schema requirements
- Provides parameter validation and suggestions

### 4. Discovery Cache (DiscoveryCache)
- Caches tool matches and LLM responses
- Optimizes performance for repeated queries
- Reduces API costs

### 5. Fallback Manager (FallbackManager)
- Handles failures gracefully
- Provides alternative suggestions
- Learns from past interactions

## Search Strategies

### 1. Rule-based Search (Fast)
**Best for:** Exact tool names, common keywords, quick matches

**How it works:**
- Exact tool name matching with fuzzy fallback
- Keyword matching in tool descriptions
- Category-based classification (network, file, database)
- Typo tolerance and partial matching

### 2. Semantic Search (Intelligent)
**Best for:** Natural language queries, conceptual matches, synonyms

**How it works:**
- Converts requests to embedding vectors
- Compares with pre-computed tool embeddings
- Uses cosine similarity for matching
- Supports multiple embedding models

**Supported Models:**
- **OpenAI**: `text-embedding-3-small` (1536 dims), `text-embedding-3-large` (3072 dims)
- **Ollama**: `nomic-embed-text` (768 dims) - Recommended for local development
- **External API**: Custom embedding services
- **Fallback Models**: `all-MiniLM-L6-v2` (384 dims), `all-mpnet-base-v2` (768 dims)

### 3. LLM-based Search (Advanced)
**Best for:** Complex queries, disambiguation, multi-step reasoning

**How it works:**
- Uses AI models (OpenAI, Anthropic, Ollama) for tool selection
- Provides reasoning and confidence scores
- Handles ambiguous or complex requests
- Supports context-aware selection

### 4. Hybrid Search (Recommended) ⭐ **Enhanced**
**Best for:** Production environments, optimal accuracy, robust fallback

**Three-Layer Hybrid Matching:**
1. **Semantic Search (30% weight)** - Uses embeddings for natural language understanding
2. **Rule-Based Matching (15% weight)** - Fast keyword and pattern matching  
3. **LLM Intelligence (55% weight)** - Advanced AI reasoning and context understanding

**Sequential Processing:**
```
User Request → Semantic Search → Rule-Based Search → LLM Evaluation → Combined Scoring → Tool Selection
```

**Key Improvements:**
- ✅ **Complete Tool Coverage**: All tools are evaluated by all enabled methods
- ✅ **Optimized Weight Distribution**: LLM-First Strategy prioritizes AI intelligence (55%)
- ✅ **Cost-Effective LLM Usage**: Multi-criteria selection limits LLM to 30 tools maximum
- ✅ **Enhanced Observability**: Detailed reasoning shows contribution from each method

## Example Output

When a user requests "ping google.com", the system provides:

```json
{
  "discovery_reasoning": "Hybrid(Semantic: 0.732, Rule: 0.550, LLM: 0.900) = 0.797",
  "confidence_score": 0.7972411894798279,
  "tool_name": "check_network_connectivity",
  "parameters": {
    "host": "google.com"
  },
  "execution_result": {
    "success": true,
    "output": "PING google.com: 4 packets transmitted, 4 received"
  }
}
```

This shows:
- **Semantic**: Found good similarity (0.732)
- **Rule-based**: Matched keywords (0.550) 
- **LLM**: High confidence selection (0.900)
- **Final**: Combined weighted score (0.797)

## Natural Language Interface

### Basic Usage
Use the single `smart_tool_discovery` tool with natural language requests:

```json
{
  "name": "smart_tool_discovery",
  "arguments": {
    "request": "read the config.yaml file"
  }
}
```

**Web Dashboard Integration**: Access smart discovery through the web dashboard at `http://localhost:5173` with toggle between HTTP API and MCP protocol execution modes.

### Advanced Usage
```json
{
  "name": "smart_tool_discovery", 
  "arguments": {
    "request": "make HTTP POST request with authentication",
    "context": "API endpoint is https://api.example.com/data, use Bearer token",
    "preferred_tools": ["http_client"],
    "confidence_threshold": 0.8,
    "include_error_details": true
  }
}
```

### Multiple Tool Suggestions
```json
{
  "name": "smart_tool_discovery",
  "arguments": {
    "request": "backup files",
    "max_suggestions": 3,
    "include_reasoning": true
  }
}
```

**Response:**
```json
{
  "success": true,
  "result": {
    "suggestions": [
      {
        "tool_name": "file_backup",
        "confidence_score": 0.92,
        "reasoning": "Primary backup tool for file operations"
      },
      {
        "tool_name": "archive_files", 
        "confidence_score": 0.87,
        "reasoning": "Creates compressed archives of files"
      },
      {
        "tool_name": "sync_directory",
        "confidence_score": 0.75, 
        "reasoning": "Synchronizes files to backup location"
      }
    ]
  }
}
```

### Common Request Patterns
- **File Operations**: "read the package.json file", "write data to output.txt"
- **HTTP Requests**: "make GET request to health endpoint", "send POST with JSON data"
- **Database Operations**: "query users table for active accounts"
- **System Tasks**: "check system health", "execute build script"
- **Network Testing**: "ping google.com to test connectivity"

## Parameter Extraction

### Automatic Parameter Mapping
The system automatically extracts parameters from natural language:

**Input:** `"send a GET request to https://api.example.com/users"`

**Extracted Parameters:**
```json
{
  "method": "GET",
  "url": "https://api.example.com/users"
}
```

### Parameter Validation
The system validates extracted parameters against tool schemas:

```json
{
  "parameter_extraction": {
    "status": "success",
    "extracted_parameters": {
      "url": "https://api.example.com/users",
      "method": "GET"
    },
    "validation_results": {
      "url": "valid",
      "method": "valid"
    },
    "missing_required": [],
    "suggestions": []
  }
}
```

### Parameter Suggestions
When parameters are missing or invalid:

```json
{
  "parameter_extraction": {
    "status": "partial_success", 
    "extracted_parameters": {
      "url": "api.example.com"
    },
    "validation_results": {
      "url": "invalid - missing protocol"
    },
    "missing_required": ["method"],
    "suggestions": [
      "Add protocol to URL (http:// or https://)",
      "Specify HTTP method (GET, POST, PUT, DELETE)"
    ]
  }
}
```

## Configuration Modes

### Mode Selection
```yaml
smart_discovery:
  tool_selection_mode: "hybrid"  # Choose your strategy
```

**Available modes:**
- `"rule_based"` - Fast keyword matching only
- `"semantic_based"` - Vector similarity only
- `"llm_based"` - AI-powered selection only
- `"hybrid"` - Intelligent combination (recommended)

### Complete Configuration
```yaml
smart_discovery:
  enabled: true
  tool_selection_mode: "hybrid"
  default_confidence_threshold: 0.7
  max_tools_to_consider: 10
  max_high_quality_matches: 5
  high_quality_threshold: 0.95
  use_fuzzy_matching: true
  
  # LLM Tool Selection
  llm_tool_selection:
    enabled: true
    provider: "openai"
    model: "gpt-4o-mini"
    api_key_env: "OPENAI_API_KEY"
    timeout: 30
    max_retries: 3
    batch_size: 15
    max_context_tokens: 4000
  
  # LLM Parameter Mapping
  llm_mapper:
    enabled: true
    provider: "openai"
    model: "gpt-4o-mini"
    api_key_env: "OPENAI_API_KEY"
    timeout: 30
    max_retries: 3
  
  # Performance Caching
  cache:
    enabled: true
    max_tool_matches: 1000
    tool_match_ttl: 3600
    max_llm_responses: 500
    llm_response_ttl: 1800
    max_registry_entries: 100
    registry_ttl: 300
  
  # Fallback Strategy
  fallback:
    enabled: true
    max_suggestions: 3
    enable_learning: true
    enable_keyword_fallback: true
    enable_category_fallback: true
    enable_partial_match_fallback: true
  
  # Semantic Search
  semantic_search:
    enabled: true
    model_name: "openai:text-embedding-3-small"
    similarity_threshold: 0.55
    max_results: 10
    
    # Storage Configuration
    storage:
      embeddings_file: "./data/embeddings/tool_embeddings.bin"
      metadata_file: "./data/embeddings/tool_metadata.json"
      hash_file: "./data/embeddings/content_hashes.json"
      backup_count: 3
      auto_backup: true
      compression: true
    
    # Model Configuration
    model:
      cache_dir: "./data/models"
      device: "cpu"
      max_sequence_length: 512
      batch_size: 32
      normalize_embeddings: true
      
    # Performance Configuration
    performance:
      lazy_loading: true
      embedding_cache_size: 1000
      parallel_processing: true
      worker_threads: 4

# Embedding Management
embedding_manager:
  batch_size: 10
  check_interval_seconds: 300
  preserve_user_settings: true
  background_monitoring: true
  auto_save: true

# Visibility Configuration
visibility:
  hide_individual_tools: true      # Hide individual tools when smart discovery is enabled
  expose_smart_discovery_only: true # Only expose smart_tool_discovery
  allow_override: true             # Allow individual tools to override hidden setting
  default_hidden: false            # Default hidden state for new tools
```

### Environment Variables
Configure LLM providers and semantic search settings:

```bash
# LLM API Keys
export OPENAI_API_KEY="your-openai-key"
export ANTHROPIC_API_KEY="your-anthropic-key"
export OLLAMA_BASE_URL="http://localhost:11434"

# Semantic Search Configuration
export MAGICTUNNEL_SEMANTIC_MODEL="openai:text-embedding-3-small"  # Override embedding model
export MAGICTUNNEL_EMBEDDING_FILE="./custom/path/embeddings.bin"    # Custom embedding file path
export MAGICTUNNEL_DISABLE_SEMANTIC="false"                         # Enable/disable semantic search

# For Ollama models
export OLLAMA_BASE_URL="http://localhost:11434"

# For custom embedding API
export EMBEDDING_API_URL="http://your-server:8080"
```

## Performance Optimization

### Caching Strategy
The system uses multi-level caching:

1. **Tool Match Cache** - Caches search results
2. **LLM Response Cache** - Caches AI responses
3. **Registry Cache** - Caches tool registry data
4. **Embedding Cache** - Caches embedding vectors

### Cost Optimization
- **Limited LLM Scope**: Maximum 30 tools evaluated by LLM
- **Strategic Selection**: Multi-criteria approach balances cost vs coverage
- **Caching**: LLM responses cached for repeated queries

### Speed
- **Sequential Processing**: Optimized for accuracy over pure speed
- **Embedding Cache**: Fast semantic search with pre-computed embeddings
- **Rule-Based Speed**: Instant exact matching for common patterns

## Visibility Management CLI

MagicTunnel includes a powerful CLI tool (`magictunnel-visibility`) for managing tool visibility:

```bash
# Check current visibility status
cargo run --bin magictunnel-visibility -- -c config.yaml status

# Show detailed status with per-file breakdown
cargo run --bin magictunnel-visibility -- -c config.yaml status --detailed

# Hide specific tools
cargo run --bin magictunnel-visibility -- -c config.yaml hide-tool tool_name

# Show specific tools
cargo run --bin magictunnel-visibility -- -c config.yaml show-tool tool_name

# Hide all tools in a capability file
cargo run --bin magictunnel-visibility -- -c config.yaml hide-file capabilities/web/http_client.yaml

# Show all tools in a capability file
cargo run --bin magictunnel-visibility -- -c config.yaml show-file capabilities/web/http_client.yaml

# Hide all tools globally
cargo run --bin magictunnel-visibility -- -c config.yaml hide-all

# Show all tools globally
cargo run --bin magictunnel-visibility -- -c config.yaml show-all
```

### Per-Tool Visibility Control
Individual tools can be marked as hidden in capability files:

```yaml
tools:
  - name: "internal_debug_tool"
    description: "Internal debugging tool"
    hidden: true  # Hidden from main tool list, available through discovery
    inputSchema:
      type: "object"
      properties:
        debug_level:
          type: "string"
          description: "Debug level (info, debug, trace)"
```

## Error Handling and Fallbacks

### Graceful Degradation
The system provides intelligent fallbacks:

1. **LLM Failure** → Falls back to semantic + rule-based search
2. **Semantic Search Failure** → Falls back to rule-based search
3. **No High-Confidence Match** → Provides suggestions with reasoning
4. **Parameter Extraction Failure** → Requests clarification with examples

### Error Response Format
```json
{
  "success": false,
  "error": {
    "code": "TOOL_NOT_FOUND",
    "message": "No suitable tool found for request",
    "details": {
      "search_method": "hybrid",
      "searched_tools": 45,
      "highest_confidence": 0.45,
      "threshold": 0.7
    },
    "suggestions": [
      "Try being more specific about the action you want to perform",
      "Check if the tool exists: list available tools",
      "Lower confidence threshold for more permissive matching"
    ],
    "fallback_options": [
      {
        "tool_name": "execute_command",
        "confidence": 0.45,
        "reasoning": "Generic command execution tool"
      }
    ]
  }
}
```

## Embedding Pre-Generation for Faster Startup

For production deployments, pre-generate embeddings to eliminate startup delays. Multiple embedding models are supported:

### Ollama (Recommended for Local Development)
```bash
# First time setup - install Ollama and pull embedding model
ollama pull nomic-embed-text

# Pre-generate embeddings with real semantic understanding
make pregenerate-embeddings-ollama    # Real embeddings (768 dimensions)

# Run server with real semantic search
make run-release-ollama               # RECOMMENDED FOR LOCAL DEVELOPMENT
```

### Cloud Models (API Key Required)
```bash
# Pre-generate embeddings
make pregenerate-embeddings-openai OPENAI_API_KEY=your-key     # OpenAI (1536 dimensions)
make pregenerate-embeddings-external EMBEDDING_API_URL=http://your-server:8080  # Custom API

# Run server  
make run-release-semantic OPENAI_API_KEY=your-key             # RECOMMENDED FOR PRODUCTION
make run-release-external EMBEDDING_API_URL=http://your-server:8080  # Custom API
```

**Model Comparison:**

| Model | Dimensions | Speed | Quality | API Key | Status | Best For |
|-------|------------|-------|---------|---------|--------|----------|
| `ollama:nomic-embed-text` | 768 | ⚡⚡ | ⭐⭐⭐⭐ | ❌ | ✅ **Real embeddings** | **🏆 Local development (recommended)** |
| `openai:text-embedding-3-small` | 1536 | ⚡⚡⚡ | ⭐⭐⭐⭐⭐ | ✅ | ✅ **Real embeddings** | **🏆 Production (recommended)** |
| `openai:text-embedding-3-large` | 3072 | ⚡⚡ | ⭐⭐⭐⭐⭐ | ✅ | ✅ **Real embeddings** | Premium production |
| `external:api` | Variable | ⚡⚡ | ⭐⭐⭐⭐ | ❌ | ✅ **Real embeddings** | Custom embedding services |
| `all-MiniLM-L6-v2` | 384 | ⚡⚡⚡ | 🚫 | ❌ | ⚠️ **Hash fallback** | Development/testing only |
| `all-mpnet-base-v2` | 768 | ⚡⚡ | 🚫 | ❌ | ⚠️ **Hash fallback** | Development/testing only |

**Benefits:**
- ⚡ **Faster Startup**: Embeddings pre-computed, no runtime generation delays
- 🚀 **Production Ready**: Perfect for containerized deployments and CI/CD
- 📊 **Detailed Reports**: Shows created/updated/failed embedding counts
- 🔧 **Flexible Models**: Use Ollama (local), OpenAI (cloud), or custom APIs
- 💰 **Cost Control**: Ollama provides local embeddings with no API costs
- 🧠 **Real Semantic Understanding**: Ollama and OpenAI provide genuine semantic embeddings

## Benefits

1. **Clean User Experience**: Users see a clean interface without tool clutter
2. **Natural Language**: Express requests in plain English instead of learning tool names
3. **Intelligent Discovery**: Automatically finds the best tool for any request
4. **Parameter Mapping**: Automatically maps natural language to tool parameters
5. **Scalable**: Works with any number of tools (tested with 83+ tools)
6. **High Performance**: Fast local discovery with LLM-powered parameter extraction
7. **Flexible Management**: Easy CLI-based visibility control
8. **Developer Friendly**: Simple configuration and backward compatibility

## Troubleshooting

**Common Issues:**

1. **"No suitable tool found"**
   - Try lowering confidence threshold: `"confidence_threshold": 0.5`
   - Be more specific: "read the config.yaml file" vs "read file"
   - Add context: Provide more details about your goal

2. **"Missing required parameter"**
   - Include specific details: "read the config.yaml file from /app/config/"
   - Use context field: `"context": "Application startup configuration"`

3. **"Ambiguous request"**
   - Be more specific: "search for error messages in log files" vs "search"
   - Use preferred tools: `"preferred_tools": ["grep_tool", "search_files"]`

## API Reference

### Smart Discovery Endpoints
- `POST /v1/mcp/call` - Execute smart tool discovery
- `GET /v1/discovery/stats` - Get discovery statistics
- `POST /v1/embeddings/sync` - Force embedding synchronization
- `GET /health/semantic` - Semantic search health check
- `POST /dashboard/api/mcp/execute` - Web dashboard MCP execution endpoint
- `GET /dashboard` - Web dashboard interface with MCP mode toggle

### Configuration Endpoints
- `GET /v1/config/semantic` - Get semantic search configuration
- `PUT /v1/config/semantic` - Update semantic search configuration

### WebSocket Integration
Smart Discovery works seamlessly with WebSocket connections:

```javascript
// WebSocket client example
const ws = new WebSocket('ws://localhost:8080/mcp/ws');

ws.send(JSON.stringify({
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "smart_tool_discovery",
    "arguments": {
      "request": "check server health"
    }
  }
}));
```