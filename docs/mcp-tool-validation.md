# Runtime Tool Validation & Security Sandboxing

MagicTunnel implements comprehensive runtime tool validation and security sandboxing according to MCP 2025-06-18 specification with classification-based policies and performance optimization.

## Overview

The Runtime Tool Validation System provides:
- **Security Classifications**: Safe, Restricted, Privileged, Dangerous, Blocked
- **Sandbox Policies**: Resource limits, network restrictions, filesystem controls
- **Runtime Validation**: Schema validation, parameter sanitization, security analysis
- **Performance Optimization**: LRU caching, batch validation, configurable thresholds
- **Comprehensive Statistics**: Validation metrics, performance tracking, security insights

## Implementation

### Core Components

- **File**: `src/mcp/tool_validation.rs`
- **Integration**: `src/mcp/server.rs` (RuntimeToolValidator)
- **Configuration**: `ValidationConfig` with security policies and performance settings

### Key Features

1. **Security Classifications**
   ```rust
   pub enum SecurityClassification {
       Safe,         // No restrictions, fully trusted
       Restricted,   // Limited capabilities, basic sandboxing
       Privileged,   // Requires special permissions
       Dangerous,    // High-risk operations, strict sandboxing
       Blocked,      // Completely blocked, cannot execute
   }
   ```

2. **Sandbox Policies**
   ```rust
   pub struct SandboxPolicy {
       pub max_execution_time_seconds: u64,
       pub max_memory_usage_mb: u64,
       pub max_cpu_usage_percent: u8,
       pub allowed_network_hosts: Vec<String>,
       pub blocked_network_ports: Vec<u16>,
       pub allowed_filesystem_paths: Vec<String>,
       pub blocked_filesystem_patterns: Vec<String>,
       pub environment_variables: HashMap<String, String>,
       pub max_output_size_kb: u64,
       pub allow_subprocess_execution: bool,
   }
   ```

3. **Validation Results**
   ```rust
   pub struct ValidationResult {
       pub is_valid: bool,
       pub classification: SecurityClassification,
       pub sandbox_policy: SandboxPolicy,
       pub warnings: Vec<String>,
       pub errors: Vec<String>,
       pub validation_time_ms: u64,
       pub cached: bool,
   }
   ```

## API Usage

### Create Tool Validator
```rust
let config = ValidationConfig {
    enabled: true,
    strict_mode: false,
    cache_enabled: true,
    cache_max_size: 10000,
    cache_ttl_seconds: 3600,
    default_classification: SecurityClassification::Restricted,
    validation_timeout_seconds: 30,
    enable_parameter_sanitization: true,
    enable_schema_validation: true,
    enable_security_analysis: true,
};

let tool_validator = RuntimeToolValidator::new(config);
```

### Basic Tool Validation
```rust
// Define tool
let tool_definition = json!({
    "name": "file_reader",
    "description": "Read files from the filesystem",
    "input_schema": {
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "File path to read"
            }
        },
        "required": ["path"]
    }
});

// Validation context
let context = json!({
    "user_id": "user123",
    "permissions": ["file_read"],
    "execution_environment": "sandboxed"
}).as_object().unwrap().clone().into_iter().collect();

// Validate tool
let result = tool_validator.validate_tool(
    "file_reader",
    &tool_definition,
    context
).await?;

if result.is_valid {
    println!("Tool is valid with classification: {:?}", result.classification);
    println!("Sandbox policy: CPU limit {}%", result.sandbox_policy.max_cpu_usage_percent);
} else {
    println!("Tool validation failed:");
    for error in result.errors {
        println!("  - {}", error);
    }
}
```

### Advanced Validation with Custom Policies
```rust
// Create custom sandbox policy for dangerous operations
let dangerous_policy = SandboxPolicy {
    max_execution_time_seconds: 300,
    max_memory_usage_mb: 512,
    max_cpu_usage_percent: 50,
    allowed_network_hosts: vec!["api.internal.com".to_string()],
    blocked_network_ports: vec![22, 23, 3389], // Block SSH, Telnet, RDP
    allowed_filesystem_paths: vec!["/tmp/safe/".to_string()],
    blocked_filesystem_patterns: vec![
        "/etc/*".to_string(),
        "/root/*".to_string(),
        "*.exe".to_string(),
    ],
    environment_variables: HashMap::from([
        ("SANDBOX_MODE".to_string(), "true".to_string()),
        ("MAX_FILE_SIZE".to_string(), "1048576".to_string()),
    ]),
    max_output_size_kb: 1024,
    allow_subprocess_execution: false,
};

// Update sandbox policy for dangerous classification
tool_validator.update_sandbox_policy(
    SecurityClassification::Dangerous,
    dangerous_policy
).await?;

// Validate tool with updated policy
let result = tool_validator.validate_tool(
    "system_command",
    &dangerous_tool_definition,
    context
).await?;
```

### Batch Validation
```rust
// Validate multiple tools efficiently
let tools = vec![
    ("ping", &ping_tool_def),
    ("curl", &curl_tool_def),
    ("file_read", &file_read_def),
];

let mut results = Vec::new();
for (name, definition) in tools {
    let result = tool_validator.validate_tool(name, definition, context.clone()).await?;
    results.push((name, result));
}

// Process results
for (name, result) in results {
    match result.classification {
        SecurityClassification::Safe => {
            println!("✅ {} - Safe to execute", name);
        }
        SecurityClassification::Restricted => {
            println!("⚠️  {} - Restricted execution", name);
        }
        SecurityClassification::Dangerous => {
            println!("🚨 {} - Dangerous - requires approval", name);
        }
        SecurityClassification::Blocked => {
            println!("❌ {} - Blocked from execution", name);
        }
        _ => {}
    }
}
```

## Configuration

### Basic Configuration
```yaml
# magictunnel-config.yaml
mcp_2025_security:
  tool_validation:
    enabled: true
    strict_mode: false
    cache_enabled: true
    cache_max_size: 10000
    cache_ttl_seconds: 3600
    default_classification: "Restricted"
    validation_timeout_seconds: 30
    enable_parameter_sanitization: true
    enable_schema_validation: true
    enable_security_analysis: true
```

### Advanced Configuration with Custom Policies
```yaml
mcp_2025_security:
  tool_validation:
    enabled: true
    strict_mode: true
    cache_enabled: true
    cache_max_size: 50000
    cache_ttl_seconds: 7200
    default_classification: "Restricted"
    validation_timeout_seconds: 60
    enable_parameter_sanitization: true
    enable_schema_validation: true
    enable_security_analysis: true
    
    # Custom sandbox policies
    sandbox_policies:
      Safe:
        max_execution_time_seconds: 60
        max_memory_usage_mb: 256
        max_cpu_usage_percent: 80
        max_output_size_kb: 512
        allow_subprocess_execution: false
        
      Restricted:
        max_execution_time_seconds: 300
        max_memory_usage_mb: 512
        max_cpu_usage_percent: 60
        allowed_network_hosts: ["api.internal.com", "safe-api.com"]
        blocked_network_ports: [22, 23, 3389]
        allowed_filesystem_paths: ["/tmp/sandbox/", "/data/readonly/"]
        max_output_size_kb: 1024
        allow_subprocess_execution: false
        
      Dangerous:
        max_execution_time_seconds: 120
        max_memory_usage_mb: 128
        max_cpu_usage_percent: 30
        allowed_network_hosts: []
        blocked_network_ports: [1, 65535]  # Block all ports
        allowed_filesystem_paths: ["/tmp/isolated/"]
        blocked_filesystem_patterns: ["/*", "*.exe", "*.sh"]
        max_output_size_kb: 256
        allow_subprocess_execution: false
```

## Security Classifications

### Classification Rules

1. **Safe Classification**
   - Read-only operations
   - No network access
   - No filesystem writes
   - No subprocess execution
   - Examples: `get_time`, `calculate`, `format_text`

2. **Restricted Classification**
   - Limited network access to approved hosts
   - Controlled filesystem access
   - Resource-limited execution
   - Examples: `fetch_api_data`, `read_config_file`, `send_notification`

3. **Privileged Classification**
   - Requires special user permissions
   - Access to sensitive resources
   - Enhanced logging and auditing
   - Examples: `manage_users`, `deploy_service`, `access_database`

4. **Dangerous Classification**
   - High-risk operations
   - Strict sandboxing required
   - Manual approval workflows
   - Examples: `execute_shell`, `modify_system`, `delete_files`

5. **Blocked Classification**
   - Completely prohibited
   - No execution allowed
   - Security policy violation
   - Examples: `format_disk`, `install_malware`, `bypass_security`

### Custom Classification Logic
```rust
impl RuntimeToolValidator {
    fn classify_tool(&self, tool_name: &str, definition: &Value) -> SecurityClassification {
        // Check for dangerous patterns
        if self.contains_dangerous_patterns(tool_name, definition) {
            return SecurityClassification::Blocked;
        }
        
        // Check for system-level operations
        if self.is_system_operation(tool_name, definition) {
            return SecurityClassification::Dangerous;
        }
        
        // Check for privileged operations
        if self.requires_privileges(tool_name, definition) {
            return SecurityClassification::Privileged;
        }
        
        // Check for network or filesystem access
        if self.accesses_external_resources(tool_name, definition) {
            return SecurityClassification::Restricted;
        }
        
        // Default to safe for simple operations
        SecurityClassification::Safe
    }
    
    fn contains_dangerous_patterns(&self, tool_name: &str, definition: &Value) -> bool {
        let dangerous_keywords = [
            "delete", "remove", "format", "destroy", "kill", "terminate",
            "install", "uninstall", "modify", "change", "update", "upgrade",
            "sudo", "admin", "root", "privilege", "escalate"
        ];
        
        let content = format!("{} {}", tool_name, definition.to_string()).to_lowercase();
        dangerous_keywords.iter().any(|keyword| content.contains(keyword))
    }
}
```

## Statistics and Monitoring

### Available Metrics
```rust
pub struct ValidationStats {
    pub total_validations: usize,
    pub cached_validations: usize,
    pub validations_by_classification: HashMap<String, usize>,
    pub validation_errors: usize,
    pub average_validation_time_ms: f64,
    pub cache_hit_rate: f64,
    pub blocked_tools: usize,
    pub policy_violations: usize,
}
```

### Get Statistics
```rust
let stats = tool_validator.get_stats().await;
println!("Total validations: {}", stats.total_validations);
println!("Cache hit rate: {:.2}%", stats.cache_hit_rate * 100.0);
println!("Average validation time: {:.2}ms", stats.average_validation_time_ms);

// Validations by classification
for (classification, count) in &stats.validations_by_classification {
    println!("{}: {}", classification, count);
}

// Security metrics
println!("Blocked tools: {}", stats.blocked_tools);
println!("Policy violations: {}", stats.policy_violations);
```

## MCP Server Integration

### Access via MCP Server
```rust
// Validate tool before execution
let validation_result = mcp_server.validate_tool(
    tool_name,
    &tool_definition,
    context
).await?;

if !validation_result.is_valid {
    return Err(ProxyError::mcp("Tool validation failed"));
}

// Execute tool with sandbox policy
let result = execute_tool_with_sandbox(
    tool_call,
    &validation_result.sandbox_policy
).await?;

// Get validation statistics
let stats = mcp_server.get_tool_validation_stats().await;

// Clear validation cache
mcp_server.clear_tool_validation_cache().await?;

// Update sandbox policies
mcp_server.update_sandbox_policy(
    SecurityClassification::Dangerous,
    custom_policy
).await?;
```

## Integration with Tool Execution

### Pre-execution Validation
```rust
pub async fn execute_tool_safely(
    tool_validator: &RuntimeToolValidator,
    tool_call: &ToolCall,
    tool_definition: &Value,
) -> Result<ToolResult, ProxyError> {
    // Validate tool before execution
    let context = extract_execution_context(tool_call);
    let validation_result = tool_validator.validate_tool(
        &tool_call.name,
        tool_definition,
        context
    ).await?;

    // Check validation result
    if !validation_result.is_valid {
        return Ok(ToolResult::error(
            format!("Tool validation failed: {}", validation_result.errors.join(", "))
        ));
    }

    // Check if tool is blocked
    if matches!(validation_result.classification, SecurityClassification::Blocked) {
        return Ok(ToolResult::error("Tool execution is blocked by security policy"));
    }

    // Execute with sandbox policy
    match validation_result.classification {
        SecurityClassification::Safe => {
            execute_tool_unrestricted(tool_call).await
        }
        SecurityClassification::Restricted | SecurityClassification::Privileged => {
            execute_tool_with_sandbox(tool_call, &validation_result.sandbox_policy).await
        }
        SecurityClassification::Dangerous => {
            // Require manual approval for dangerous operations
            request_approval_and_execute(tool_call, &validation_result.sandbox_policy).await
        }
        SecurityClassification::Blocked => {
            Ok(ToolResult::error("Tool execution blocked"))
        }
    }
}
```

### Sandboxed Execution
```rust
pub async fn execute_tool_with_sandbox(
    tool_call: &ToolCall,
    sandbox_policy: &SandboxPolicy,
) -> Result<ToolResult, ProxyError> {
    // Create execution environment
    let mut command = tokio::process::Command::new("sandbox-runner");
    
    // Apply resource limits
    command.env("MAX_MEMORY_MB", sandbox_policy.max_memory_usage_mb.to_string());
    command.env("MAX_CPU_PERCENT", sandbox_policy.max_cpu_usage_percent.to_string());
    command.env("MAX_EXECUTION_TIME", sandbox_policy.max_execution_time_seconds.to_string());
    
    // Set allowed filesystem paths
    if !sandbox_policy.allowed_filesystem_paths.is_empty() {
        command.env("ALLOWED_PATHS", sandbox_policy.allowed_filesystem_paths.join(":"));
    }
    
    // Set network restrictions
    if !sandbox_policy.allowed_network_hosts.is_empty() {
        command.env("ALLOWED_HOSTS", sandbox_policy.allowed_network_hosts.join(","));
    }
    
    // Apply environment variables
    for (key, value) in &sandbox_policy.environment_variables {
        command.env(key, value);
    }
    
    // Execute with timeout
    let execution_timeout = Duration::from_secs(sandbox_policy.max_execution_time_seconds);
    let result = tokio::time::timeout(execution_timeout, command.output()).await;
    
    match result {
        Ok(Ok(output)) => {
            // Check output size limits
            if output.stdout.len() > (sandbox_policy.max_output_size_kb as usize * 1024) {
                return Ok(ToolResult::error("Output size exceeds sandbox limits"));
            }
            
            // Process successful execution
            let output_str = String::from_utf8_lossy(&output.stdout);
            Ok(ToolResult::success(json!({"output": output_str})))
        }
        Ok(Err(e)) => {
            Ok(ToolResult::error(format!("Sandbox execution failed: {}", e)))
        }
        Err(_) => {
            Ok(ToolResult::error("Tool execution timed out"))
        }
    }
}
```

## Parameter Sanitization

### Input Sanitization
```rust
impl RuntimeToolValidator {
    fn sanitize_parameters(&self, parameters: &mut Value) -> Result<(), ValidationError> {
        match parameters {
            Value::Object(obj) => {
                for (key, value) in obj.iter_mut() {
                    self.sanitize_parameter_value(key, value)?;
                }
            }
            _ => return Err(ValidationError::InvalidParameterFormat),
        }
        Ok(())
    }
    
    fn sanitize_parameter_value(&self, key: &str, value: &mut Value) -> Result<(), ValidationError> {
        match value {
            Value::String(s) => {
                // Remove potentially dangerous characters
                *s = self.sanitize_string(s);
                
                // Check for injection patterns
                if self.contains_injection_patterns(s) {
                    return Err(ValidationError::PotentialInjection(key.to_string()));
                }
                
                // Path traversal protection
                if key.contains("path") || key.contains("file") {
                    *s = self.sanitize_file_path(s)?;
                }
                
                // Command injection protection
                if key.contains("command") || key.contains("exec") {
                    return Err(ValidationError::UnsafeParameter(key.to_string()));
                }
            }
            Value::Object(obj) => {
                for (nested_key, nested_value) in obj.iter_mut() {
                    self.sanitize_parameter_value(nested_key, nested_value)?;
                }
            }
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    if let Value::String(s) = item {
                        *s = self.sanitize_string(s);
                    }
                }
            }
            _ => {} // Numbers, booleans, null are safe
        }
        Ok(())
    }
    
    fn sanitize_string(&self, input: &str) -> String {
        input
            .replace("../", "")         // Path traversal
            .replace("..\\", "")        // Windows path traversal
            .replace(";", "")           // Command separator
            .replace("|", "")           // Pipe operator
            .replace("&", "")           // Command separator
            .replace("`", "")           // Command substitution
            .replace("$", "")           // Variable expansion
            .chars()
            .filter(|c| c.is_alphanumeric() || " .-_/".contains(*c))
            .collect()
    }
    
    fn contains_injection_patterns(&self, input: &str) -> bool {
        let dangerous_patterns = [
            "'; DROP TABLE",
            "'; DELETE FROM",
            "<script>",
            "javascript:",
            "eval(",
            "exec(",
            "system(",
            "${",
            "#{",
        ];
        
        let lower_input = input.to_lowercase();
        dangerous_patterns.iter().any(|pattern| lower_input.contains(&pattern.to_lowercase()))
    }
    
    fn sanitize_file_path(&self, path: &str) -> Result<String, ValidationError> {
        // Normalize path
        let normalized = std::path::Path::new(path)
            .canonicalize()
            .map_err(|_| ValidationError::InvalidPath(path.to_string()))?;
        
        // Check if path is within allowed directories
        let allowed_prefixes = ["/tmp/sandbox/", "/data/safe/", "/workspace/"];
        let path_str = normalized.to_string_lossy();
        
        if !allowed_prefixes.iter().any(|prefix| path_str.starts_with(prefix)) {
            return Err(ValidationError::UnauthorizedPath(path.to_string()));
        }
        
        Ok(path_str.to_string())
    }
}
```

## Schema Validation

### JSON Schema Validation
```rust
impl RuntimeToolValidator {
    fn validate_tool_schema(&self, definition: &Value) -> Result<(), ValidationError> {
        // Check required fields
        let obj = definition.as_object()
            .ok_or(ValidationError::InvalidToolDefinition)?;
        
        // Validate name
        let name = obj.get("name")
            .and_then(|v| v.as_str())
            .ok_or(ValidationError::MissingFieldError("name".to_string()))?;
        
        if name.is_empty() || !self.is_valid_tool_name(name) {
            return Err(ValidationError::InvalidToolName(name.to_string()));
        }
        
        // Validate description
        let description = obj.get("description")
            .and_then(|v| v.as_str())
            .ok_or(ValidationError::MissingFieldError("description".to_string()))?;
        
        if description.len() < 10 {
            return Err(ValidationError::InsufficientDescription);
        }
        
        // Validate input schema
        if let Some(input_schema) = obj.get("input_schema") {
            self.validate_input_schema(input_schema)?;
        }
        
        Ok(())
    }
    
    fn validate_input_schema(&self, schema: &Value) -> Result<(), ValidationError> {
        let schema_obj = schema.as_object()
            .ok_or(ValidationError::InvalidInputSchema)?;
        
        // Check schema type
        let schema_type = schema_obj.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("object");
        
        if schema_type != "object" {
            return Err(ValidationError::UnsupportedSchemaType(schema_type.to_string()));
        }
        
        // Validate properties
        if let Some(properties) = schema_obj.get("properties") {
            let props = properties.as_object()
                .ok_or(ValidationError::InvalidSchemaProperties)?;
            
            for (prop_name, prop_def) in props {
                self.validate_property_definition(prop_name, prop_def)?;
            }
        }
        
        Ok(())
    }
    
    fn validate_property_definition(&self, name: &str, definition: &Value) -> Result<(), ValidationError> {
        let prop_obj = definition.as_object()
            .ok_or(ValidationError::InvalidPropertyDefinition(name.to_string()))?;
        
        // Check for dangerous property names
        let dangerous_props = ["password", "secret", "token", "key", "credential"];
        if dangerous_props.iter().any(|&dangerous| name.to_lowercase().contains(dangerous)) {
            return Err(ValidationError::DangerousProperty(name.to_string()));
        }
        
        // Validate property type
        if let Some(prop_type) = prop_obj.get("type").and_then(|v| v.as_str()) {
            let allowed_types = ["string", "number", "integer", "boolean", "array", "object"];
            if !allowed_types.contains(&prop_type) {
                return Err(ValidationError::UnsupportedPropertyType(prop_type.to_string()));
            }
        }
        
        Ok(())
    }
    
    fn is_valid_tool_name(&self, name: &str) -> bool {
        // Tool name should be alphanumeric with underscores/hyphens
        name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') &&
        !name.is_empty() &&
        name.len() <= 64
    }
}
```

## Performance Optimization

### Caching Strategy
```rust
use lru::LruCache;

impl RuntimeToolValidator {
    fn new_with_cache(config: ValidationConfig) -> Self {
        let cache = if config.cache_enabled {
            Some(Arc::new(RwLock::new(LruCache::new(config.cache_max_size))))
        } else {
            None
        };
        
        Self {
            config,
            cache,
            statistics: Arc::new(RwLock::new(ValidationStats::default())),
            sandbox_policies: Arc::new(RwLock::new(Self::default_sandbox_policies())),
        }
    }
    
    async fn get_cached_result(&self, cache_key: &str) -> Option<ValidationResult> {
        if let Some(cache) = &self.cache {
            let cache_guard = cache.read().await;
            if let Some((result, timestamp)) = cache_guard.peek(cache_key) {
                let age = Utc::now().timestamp() - timestamp;
                if age < self.config.cache_ttl_seconds as i64 {
                    let mut result = result.clone();
                    result.cached = true;
                    return Some(result);
                }
            }
        }
        None
    }
    
    async fn cache_result(&self, cache_key: String, result: &ValidationResult) {
        if let Some(cache) = &self.cache {
            let mut cache_guard = cache.write().await;
            cache_guard.put(cache_key, (result.clone(), Utc::now().timestamp()));
        }
    }
    
    fn generate_cache_key(&self, tool_name: &str, definition: &Value, context: &HashMap<String, Value>) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        tool_name.hash(&mut hasher);
        definition.to_string().hash(&mut hasher);
        format!("{:?}", context).hash(&mut hasher); // Simple context serialization
        
        format!("tool_validation_{:x}", hasher.finish())
    }
}
```

### Batch Validation
```rust
impl RuntimeToolValidator {
    pub async fn validate_tools_batch(
        &self,
        tools: Vec<(&str, &Value, HashMap<String, Value>)>,
    ) -> Result<Vec<ValidationResult>, ProxyError> {
        let batch_size = 10; // Process in chunks to avoid overwhelming the system
        let mut results = Vec::with_capacity(tools.len());
        
        for chunk in tools.chunks(batch_size) {
            let chunk_futures: Vec<_> = chunk.iter()
                .map(|(name, definition, context)| {
                    self.validate_tool(name, definition, context.clone())
                })
                .collect();
            
            let chunk_results = futures::future::join_all(chunk_futures).await;
            
            for result in chunk_results {
                results.push(result?);
            }
            
            // Small delay between batches to prevent resource exhaustion
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        Ok(results)
    }
}
```

## Best Practices

### 1. Security-First Validation
```rust
// Good: Default to restrictive, explicitly allow
fn classify_tool_securely(tool_name: &str, definition: &Value) -> SecurityClassification {
    // Start with most restrictive
    let mut classification = SecurityClassification::Blocked;
    
    // Explicitly allow known safe operations
    if is_read_only_operation(tool_name, definition) {
        classification = SecurityClassification::Safe;
    } else if is_network_operation_with_approved_hosts(tool_name, definition) {
        classification = SecurityClassification::Restricted;
    }
    
    classification
}

// Avoid: Default to permissive
// let classification = SecurityClassification::Safe; // Too permissive
```

### 2. Comprehensive Parameter Validation
```rust
// Good: Validate all aspects of parameters
fn validate_parameters_thoroughly(params: &Value) -> Result<(), ValidationError> {
    // Type validation
    validate_parameter_types(params)?;
    
    // Content validation
    validate_parameter_content(params)?;
    
    // Security validation
    validate_parameter_security(params)?;
    
    // Business logic validation
    validate_parameter_business_rules(params)?;
    
    Ok(())
}

// Avoid: Minimal validation
// if params.is_object() { Ok(()) } else { Err(...) }
```

### 3. Proper Error Handling
```rust
// Good: Specific error types with context
match tool_validator.validate_tool(name, definition, context).await {
    Ok(result) if result.is_valid => {
        execute_tool_with_sandbox(tool_call, &result.sandbox_policy).await
    }
    Ok(result) => {
        log_validation_failure(&result);
        Err(ProxyError::ValidationFailed(result.errors))
    }
    Err(ProxyError::ValidationTimeout) => {
        Err(ProxyError::mcp("Tool validation timed out"))
    }
    Err(e) => {
        error!("Unexpected validation error: {}", e);
        Err(ProxyError::mcp("Tool validation failed"))
    }
}

// Avoid: Generic error handling
// if !result.is_valid { return Err(ProxyError::mcp("Invalid tool")); }
```

### 4. Performance Monitoring
```rust
// Good: Track validation performance
pub async fn validate_with_metrics(
    &self,
    tool_name: &str,
    definition: &Value,
    context: HashMap<String, Value>,
) -> Result<ValidationResult, ProxyError> {
    let start_time = Instant::now();
    
    let result = self.validate_tool(tool_name, definition, context).await;
    
    let duration = start_time.elapsed();
    self.record_validation_metrics(tool_name, duration, &result).await;
    
    if duration > Duration::from_secs(5) {
        warn!("Slow validation for tool {}: {}ms", tool_name, duration.as_millis());
    }
    
    result
}

// Avoid: No performance tracking
```

## Testing

### Unit Tests
The tool validation system includes comprehensive unit tests:
- Security classification logic
- Parameter sanitization
- Schema validation
- Sandbox policy enforcement
- Cache functionality

### Integration Tests
See `tests/mcp_2025_06_18_compliance_test.rs` for integration tests covering:
- End-to-end validation workflows
- MCP server integration
- Performance benchmarks
- Security policy enforcement

### Security Tests
```rust
#[tokio::test]
async fn test_dangerous_tool_classification() {
    let validator = RuntimeToolValidator::new(ValidationConfig::default());
    
    let dangerous_tool = json!({
        "name": "delete_all_files",
        "description": "Recursively delete all files in a directory",
        "input_schema": {
            "type": "object",
            "properties": {
                "path": {"type": "string"}
            }
        }
    });
    
    let result = validator.validate_tool(
        "delete_all_files",
        &dangerous_tool,
        HashMap::new()
    ).await.unwrap();
    
    assert!(matches!(result.classification, SecurityClassification::Blocked));
    assert!(!result.is_valid);
    assert!(!result.errors.is_empty());
}

#[tokio::test]
async fn test_parameter_injection_detection() {
    let validator = RuntimeToolValidator::new(ValidationConfig::default());
    
    let tool_with_injection = json!({
        "name": "safe_tool",
        "description": "A safe tool",
        "input_schema": {
            "type": "object",
            "properties": {
                "query": {"type": "string"}
            }
        }
    });
    
    let malicious_context = json!({
        "parameters": {
            "query": "'; DROP TABLE users; --"
        }
    }).as_object().unwrap().clone().into_iter().collect();
    
    let result = validator.validate_tool(
        "safe_tool",
        &tool_with_injection,
        malicious_context
    ).await.unwrap();
    
    assert!(!result.is_valid);
    assert!(result.errors.iter().any(|e| e.contains("injection")));
}
```

## Troubleshooting

### Common Issues

1. **High Validation Latency**
   ```rust
   // Enable caching
   ValidationConfig {
       cache_enabled: true,
       cache_max_size: 50000,
       cache_ttl_seconds: 3600,
       // ...
   }
   
   // Use batch validation for multiple tools
   let results = validator.validate_tools_batch(tools).await?;
   ```

2. **False Positive Classifications**
   ```rust
   // Customize classification rules
   validator.add_classification_override(
       "specific_tool",
       SecurityClassification::Safe
   ).await?;
   
   // Update sandbox policies
   validator.update_sandbox_policy(
       SecurityClassification::Restricted,
       custom_policy
   ).await?;
   ```

3. **Memory Usage from Cache**
   ```rust
   // Configure appropriate cache size
   ValidationConfig {
       cache_max_size: 10000, // Adjust based on available memory
       cache_ttl_seconds: 1800, // Shorter TTL for frequent changes
       // ...
   }
   
   // Clear cache periodically
   validator.clear_cache().await?;
   ```

## Related Documentation
- [Enhanced Cancellation](mcp-cancellation.md) - Tool validation with cancellation support
- [Progress Tracking](mcp-progress.md) - Validation progress monitoring
- [Security Overview](security.md) - Enterprise security features
- [MCP 2025-06-18 Specification](https://spec.modelcontextprotocol.io/specification/2025-06-18/)
- [Architecture Guide](architecture.md) - System architecture overview