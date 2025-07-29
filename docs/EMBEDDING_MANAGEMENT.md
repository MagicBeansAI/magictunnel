# Embedding Management System

## Overview

The Embedding Management System handles the complete lifecycle of tool embeddings in MagicTunnel, including automatic generation, updates, persistence, and cleanup. This system ensures that semantic search capabilities remain synchronized with tool registry changes.

## Architecture

### Core Components

1. **Embedding Manager** (`EmbeddingManager`)
   - Manages embedding lifecycle (create, update, delete)
   - Monitors tool registry changes
   - Handles background synchronization

2. **Semantic Search Service** (`SemanticSearchService`)
   - Generates embeddings using various models
   - Provides similarity search capabilities
   - Manages persistent storage

3. **Embedding Storage** (`EmbeddingStorage`)
   - Stores embeddings, metadata, and content hashes
   - Provides atomic operations and backup management
   - Supports compression and versioning

## Lifecycle Management

### Automatic Operations

The system automatically handles:

1. **Embedding Creation**
   - New tools with `enabled: true` get embeddings generated
   - Excludes `smart_tool_discovery` to prevent recursion
   - Uses configurable batch processing for performance

2. **Embedding Updates**
   - Detects content changes via hash comparison
   - Updates embeddings when tool descriptions change
   - Handles enabled/disabled state changes

3. **Embedding Removal**
   - Removes embeddings for deleted tools
   - Cleans up orphaned embeddings
   - Maintains storage integrity

4. **Background Synchronization**
   - Runs every 5 minutes by default
   - Non-blocking async processing
   - Comprehensive error handling

### Change Detection

The system uses multiple strategies to detect changes:

```rust
// Content hash-based detection
pub fn generate_content_hash(&self, tool_def: &ToolDefinition) -> String {
    // Combines name, description, enabled, hidden states
}

// State tracking
last_known_state: HashMap<String, (String, bool, bool)>
// tool_name -> (content_hash, enabled, hidden)
```

## Configuration

### Embedding Manager Configuration

```yaml
embedding_manager:
  check_interval_seconds: 300      # Background sync interval (5 minutes)
  auto_save: true                  # Automatically save after changes
  batch_size: 10                   # Process embeddings in batches
  background_monitoring: true      # Enable background monitoring
  preserve_user_settings: true    # Preserve user-disabled tools
```

### User Setting Preservation

Protects user preferences during external MCP updates:

```yaml
preserve_user_settings: true
```

**How it works:**
- Tracks user-disabled tools separately from auto-generated changes
- Only updates tools if content actually changed, not just configuration
- Implements merge strategy: preserve user config + update tool definitions
- Maintains `user_disabled_tools` HashSet for tracking

## Storage Management

### File Structure

```
data/
└── embeddings/
    ├── tool_embeddings.bin      # Binary embedding vectors
    ├── tool_metadata.json       # Tool metadata and timestamps
    ├── content_hashes.json      # Content hashes for change detection
    ├── tool_embeddings.bin.backup
    ├── tool_metadata.json.backup
    └── content_hashes.json.backup
```

### Storage Configuration

```yaml
semantic_search:
  storage:
    embeddings_file: "./data/embeddings/tool_embeddings.bin"
    metadata_file: "./data/embeddings/tool_metadata.json"
    hash_file: "./data/embeddings/content_hashes.json"
    backup_count: 3              # Number of backup files to maintain
    auto_backup: true            # Create backups before updates
    compression: true            # Enable storage compression
```

### Backup and Recovery

**Automatic Backups:**
- Created before any write operation
- Configurable backup count (default: 3)
- Atomic operations prevent corruption

**Manual Backup:**
```bash
# Create manual backup
cp data/embeddings/tool_embeddings.bin data/embeddings/tool_embeddings.bin.manual

# Restore from backup
cp data/embeddings/tool_embeddings.bin.backup data/embeddings/tool_embeddings.bin
```

## Operations and Status Tracking

### Operation Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum EmbeddingStatus {
    UpToDate,        // No changes needed
    NeedsCreation,   // New tool, needs embedding
    NeedsUpdate,     // Content changed, update required
    ShouldRemove,    // Tool deleted, remove embedding
}
```

### Operation Results

```rust
pub struct EmbeddingOperation {
    pub tool_name: String,
    pub status: EmbeddingStatus,
    pub reason: String,           // Why this operation was needed
    pub success: bool,            // Operation success/failure
    pub error: Option<String>,    // Error details if failed
}
```

### Change Summary

```rust
pub struct EmbeddingChangeSummary {
    pub total_processed: usize,
    pub created: usize,
    pub updated: usize,
    pub removed: usize,
    pub failed: usize,
    pub operations: Vec<EmbeddingOperation>,
    pub duration_ms: u64,
}
```

## API Reference

### Management Operations

#### Force Synchronization
```bash
curl -X POST http://localhost:8080/v1/embeddings/sync
```

**Response:**
```json
{
  "total_processed": 15,
  "created": 3,
  "updated": 2,
  "removed": 1,
  "failed": 0,
  "duration_ms": 1250,
  "operations": [
    {
      "tool_name": "new_ping_tool",
      "status": "NeedsCreation", 
      "reason": "new tool",
      "success": true,
      "error": null
    }
  ]
}
```

#### Get Statistics
```bash
curl http://localhost:8080/v1/embeddings/stats
```

**Response:**
```json
{
  "enabled": true,
  "model_name": "all-MiniLM-L6-v2",
  "total_embeddings": 45,
  "enabled_tools": 42,
  "hidden_tools": 8,
  "similarity_threshold": 0.7,
  "storage_dirty": false,
  "last_known_tools": 45,
  "user_disabled_tools": 3,
  "background_monitoring": true,
  "auto_save": true
}
```

#### Mark Tool as User-Disabled
```bash
curl -X POST http://localhost:8080/v1/embeddings/user-disabled \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "external_api_tool"}'
```

#### Remove User-Disabled Marking
```bash
curl -X DELETE http://localhost:3001/v1/embeddings/user-disabled/external_api_tool
```

### Health Monitoring

#### Embedding Health Check
```bash
curl http://localhost:3001/health/embeddings
```

**Response:**
```json
{
  "status": "healthy",
  "total_embeddings": 45,
  "last_sync": "2024-01-15T10:30:00Z",
  "background_monitoring": true,
  "storage_integrity": "ok"
}
```

## Performance Optimization

### Batch Processing

Configure batch sizes for optimal performance:

```yaml
embedding_manager:
  batch_size: 10              # Process 10 tools at a time
  
semantic_search:
  model:
    batch_size: 32            # Embedding generation batch size
```

### Parallel Processing

Enable parallel embedding generation:

```yaml
semantic_search:
  performance:
    parallel_processing: true
    worker_threads: 4         # Number of parallel workers
```

### Memory Management

Control memory usage:

```yaml
semantic_search:
  performance:
    lazy_loading: true        # Load embeddings on-demand
    embedding_cache_size: 1000 # Limit in-memory cache
```

## External MCP Integration

### Change Detection for External Tools

The system handles external MCP server changes:

1. **Content Comparison** - Compares tool definitions, not just existence
2. **Hash Generation** - Creates content hash for external tools  
3. **Smart Updates** - Only updates if tool definition actually changed
4. **Logging** - Logs all external MCP update decisions

### User Setting Preservation

Prevents external MCP updates from overriding user preferences:

```rust
// Check if user has disabled this tool
if user_disabled_tools.contains(tool_name) && enabled {
    debug!("Preserving user disabled setting for tool: {}", tool_name);
    return Ok(()); // Skip embedding creation
}
```

## Troubleshooting

### Common Issues

1. **Embeddings Not Updating**
   ```bash
   # Check if background monitoring is enabled
   curl http://localhost:8080/v1/embeddings/stats | grep background_monitoring
   
   # Force manual sync
   curl -X POST http://localhost:8080/v1/embeddings/sync
   ```

2. **High Memory Usage**
   ```yaml
   semantic_search:
     performance:
       lazy_loading: true
       embedding_cache_size: 500  # Reduce cache size
   ```

3. **Slow Synchronization**
   ```yaml
   embedding_manager:
     batch_size: 5              # Reduce batch size
     
   semantic_search:
     performance:
       parallel_processing: true # Enable parallel processing
       worker_threads: 8         # Increase workers
   ```

4. **Storage Corruption**
   ```bash
   # Restore from backup
   cp data/embeddings/*.backup data/embeddings/
   
   # Force regeneration
   rm data/embeddings/*
   curl -X POST http://localhost:8080/v1/embeddings/sync
   ```

### Debug Logging

Enable detailed logging:

```bash
export RUST_LOG=magictunnel::discovery::embedding_manager=debug
./target/release/magictunnel
```

### Storage Verification

Check storage integrity:

```bash
# Verify file existence
ls -la data/embeddings/

# Check file sizes
du -h data/embeddings/*

# Validate JSON files
jq . data/embeddings/tool_metadata.json
jq . data/embeddings/content_hashes.json
```

## Migration and Maintenance

### Upgrading Embedding Models

1. **Stop Background Monitoring** (temporarily):
   ```yaml
   embedding_manager:
     background_monitoring: false
   ```

2. **Clear Existing Embeddings**:
   ```bash
   rm -rf data/embeddings/*
   ```

3. **Update Model Configuration**:
   ```yaml
   semantic_search:
     model_name: "openai:text-embedding-3-large"
   ```

4. **Restart and Regenerate**:
   ```bash
   ./target/release/magictunnel
   curl -X POST http://localhost:8080/v1/embeddings/sync
   ```

### Bulk Operations

#### Regenerate All Embeddings
```bash
# Clear storage
rm -rf data/embeddings/*

# Force full regeneration
curl -X POST http://localhost:8080/v1/embeddings/regenerate-all
```

#### Export/Import Embeddings
```bash
# Export embeddings
curl http://localhost:8080/v1/embeddings/export > embeddings_backup.json

# Import embeddings
curl -X POST http://localhost:8080/v1/embeddings/import \
  -H "Content-Type: application/json" \
  -d @embeddings_backup.json
```

## Best Practices

### Production Deployment

1. **Enable Auto-Backup**: Always enable automatic backups
2. **Monitor Disk Space**: Embedding files can grow large
3. **Regular Health Checks**: Monitor embedding system health
4. **Gradual Batch Sizes**: Start with small batches, increase as needed
5. **User Setting Preservation**: Always enable for external MCP environments

### Development

1. **Disable Background Monitoring**: For faster iteration cycles
2. **Use Smaller Cache Sizes**: Reduce memory usage during development
3. **Enable Debug Logging**: For troubleshooting and understanding behavior
4. **Manual Sync**: Use manual synchronization for controlled testing

### Security

1. **File Permissions**: Restrict access to embedding storage directories
2. **Backup Security**: Secure backup files with appropriate permissions
3. **API Access**: Protect management endpoints with authentication
4. **Input Validation**: Validate all user inputs for management operations

## Monitoring and Alerts

### Key Metrics to Monitor

1. **Sync Frequency**: Background synchronization intervals
2. **Operation Success Rate**: Percentage of successful operations
3. **Storage Size**: Total size of embedding files
4. **Memory Usage**: In-memory cache utilization
5. **API Response Times**: Management operation performance

### Alerting Thresholds

```yaml
# Example monitoring thresholds
alerts:
  sync_failures: 5           # Alert if >5 consecutive sync failures
  storage_size: 10GB         # Alert if storage exceeds 10GB
  memory_usage: 80%          # Alert if memory usage >80%
  operation_time: 30s        # Alert if operations take >30s
```

This embedding management system provides robust, automatic lifecycle management for tool embeddings while preserving user preferences and maintaining data integrity.