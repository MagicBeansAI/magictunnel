//! Comprehensive audit logging for MagicTunnel
//!
//! Provides detailed logging of every communication between clients and servers,
//! similar to MCP Manager's audit capabilities for security monitoring and compliance.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use tracing::{debug, info, warn, error};

/// Configuration for audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Whether audit logging is enabled
    pub enabled: bool,
    /// What events to audit
    pub events: Vec<AuditEventType>,
    /// Where to store audit logs
    pub storage: AuditStorageConfig,
    /// How long to retain audit logs
    pub retention_days: u32,
    /// Whether to include request/response bodies
    pub include_bodies: bool,
    /// Maximum body size to log (bytes)
    pub max_body_size: usize,
    /// Whether to mask sensitive data in logs
    pub mask_sensitive_data: bool,
}

/// Types of events to audit
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// Authentication attempts
    Authentication,
    /// Authorization decisions
    Authorization,
    /// Tool calls and executions
    ToolExecution,
    /// Resource access
    ResourceAccess,
    /// Prompt access
    PromptAccess,
    /// Configuration changes
    ConfigurationChange,
    /// Error events
    Error,
    /// Security violations
    SecurityViolation,
    /// System events (startup, shutdown)
    System,
    /// All events
    All,
}

/// Audit storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuditStorageConfig {
    /// Store in local files
    File {
        /// Directory for audit logs
        directory: String,
        /// Log rotation settings
        rotation: FileRotationConfig,
    },
    /// Store in database
    Database {
        /// Database connection string
        connection_string: String,
        /// Table name for audit logs
        table_name: String,
    },
    /// Send to external logging service
    External {
        /// Service endpoint
        endpoint: String,
        /// Authentication settings
        auth: Option<ExternalAuthConfig>,
        /// Batch size for sending logs
        batch_size: usize,
        /// Flush interval in seconds
        flush_interval_seconds: u64,
    },
    /// Store in memory (for testing)
    Memory {
        /// Maximum entries to keep
        max_entries: usize,
    },
}

/// File rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRotationConfig {
    /// Rotate when file reaches this size (bytes)
    pub max_file_size: u64,
    /// Maximum number of rotated files to keep
    pub max_files: u32,
    /// Whether to compress rotated files
    pub compress: bool,
}

/// External service authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExternalAuthConfig {
    /// API key authentication
    ApiKey {
        /// Header name
        header: String,
        /// API key value
        key: String,
    },
    /// Bearer token authentication
    Bearer {
        /// Token value
        token: String,
    },
    /// Custom headers
    Custom {
        /// Headers to add
        headers: HashMap<String, String>,
    },
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry ID
    pub id: String,
    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,
    /// Type of event
    pub event_type: AuditEventType,
    /// User information
    pub user: Option<AuditUser>,
    /// Request information
    pub request: Option<AuditRequest>,
    /// Response information
    pub response: Option<AuditResponse>,
    /// Tool information (if applicable)
    pub tool: Option<AuditTool>,
    /// Resource information (if applicable)
    pub resource: Option<AuditResource>,
    /// Security context
    pub security: AuditSecurity,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Event outcome
    pub outcome: AuditOutcome,
    /// Error information (if any)
    pub error: Option<AuditError>,
}

/// User information for audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditUser {
    /// User ID
    pub id: Option<String>,
    /// User name
    pub name: Option<String>,
    /// User roles
    pub roles: Vec<String>,
    /// API key name (if using API key auth)
    pub api_key_name: Option<String>,
    /// Authentication method used
    pub auth_method: String,
}

/// Request information for audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRequest {
    /// Request ID
    pub id: Option<String>,
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Request headers (sensitive headers masked)
    pub headers: HashMap<String, String>,
    /// Request body (if enabled)
    pub body: Option<String>,
    /// Request size in bytes
    pub size: usize,
}

/// Response information for audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body (if enabled)
    pub body: Option<String>,
    /// Response size in bytes
    pub size: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Tool information for audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTool {
    /// Tool name
    pub name: String,
    /// Tool parameters (sensitive data masked)
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    /// Execution result
    pub result: Option<String>,
    /// Execution time in milliseconds
    pub execution_time_ms: Option<u64>,
    /// Whether execution was successful
    pub success: bool,
}

/// Resource information for audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResource {
    /// Resource URI
    pub uri: String,
    /// Resource type
    pub resource_type: String,
    /// Operation performed
    pub operation: String,
}

/// Security context for audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSecurity {
    /// Whether request was authenticated
    pub authenticated: bool,
    /// Whether request was authorized
    pub authorized: bool,
    /// Permissions checked
    pub permissions_checked: Vec<String>,
    /// Security policies applied
    pub policies_applied: Vec<String>,
    /// Whether content was sanitized
    pub content_sanitized: bool,
    /// Whether approval was required
    pub approval_required: bool,
}

/// Audit event outcome
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuditOutcome {
    /// Request succeeded
    Success,
    /// Request failed
    Failure,
    /// Request was blocked
    Blocked,
    /// Request requires approval
    PendingApproval,
}

/// Error information for audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Error details
    pub details: Option<String>,
    /// Stack trace (if available)
    pub stack_trace: Option<String>,
}

/// Audit service for logging and managing audit entries
pub struct AuditService {
    config: AuditConfig,
    storage: Box<dyn AuditStorage + Send + Sync>,
}

/// Trait for audit storage backends
#[async_trait::async_trait]
pub trait AuditStorage {
    /// Store an audit entry
    async fn store(&self, entry: &AuditEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    
    /// Query audit entries
    async fn query(
        &self,
        filters: &AuditQueryFilters,
    ) -> Result<Vec<AuditEntry>, Box<dyn std::error::Error + Send + Sync>>;
    
    /// Clean up old audit entries
    async fn cleanup(&self, older_than: DateTime<Utc>) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;
}

/// Filters for querying audit entries
#[derive(Debug, Clone)]
pub struct AuditQueryFilters {
    /// Start time range
    pub start_time: Option<DateTime<Utc>>,
    /// End time range
    pub end_time: Option<DateTime<Utc>>,
    /// Event types to include
    pub event_types: Option<Vec<AuditEventType>>,
    /// User ID filter
    pub user_id: Option<String>,
    /// Tool name filter
    pub tool_name: Option<String>,
    /// Outcome filter
    pub outcome: Option<AuditOutcome>,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            events: vec![AuditEventType::All],
            storage: AuditStorageConfig::Memory { max_entries: 1000 },
            retention_days: 90,
            include_bodies: false,
            max_body_size: 1024 * 1024, // 1MB
            mask_sensitive_data: true,
        }
    }
}

impl AuditService {
    /// Create a new audit service
    pub async fn new(config: AuditConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let storage = Self::create_storage(&config.storage).await?;
        
        Ok(Self {
            config,
            storage,
        })
    }
    
    /// Create storage backend based on configuration
    async fn create_storage(
        config: &AuditStorageConfig,
    ) -> Result<Box<dyn AuditStorage + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        match config {
            AuditStorageConfig::Memory { max_entries } => {
                Ok(Box::new(MemoryAuditStorage::new(*max_entries)))
            }
            AuditStorageConfig::File { directory, rotation } => {
                Ok(Box::new(FileAuditStorage::new(directory.clone(), rotation.clone()).await?))
            }
            AuditStorageConfig::Database { connection_string: _, table_name: _ } => {
                // Database storage implementation would go here
                todo!("Database audit storage not yet implemented")
            }
            AuditStorageConfig::External { endpoint: _, auth: _, batch_size: _, flush_interval_seconds: _ } => {
                // External service storage implementation would go here
                todo!("External audit storage not yet implemented")
            }
        }
    }
    
    /// Log an audit event
    pub async fn log_event(&self, mut entry: AuditEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled {
            return Ok(());
        }
        
        // Check if this event type should be audited
        if !self.should_audit_event(&entry.event_type) {
            return Ok(());
        }
        
        // Mask sensitive data if configured
        if self.config.mask_sensitive_data {
            self.mask_sensitive_data(&mut entry);
        }
        
        // Limit body size if configured
        if !self.config.include_bodies {
            if let Some(ref mut request) = entry.request {
                request.body = None;
            }
            if let Some(ref mut response) = entry.response {
                response.body = None;
            }
        } else {
            self.limit_body_size(&mut entry);
        }
        
        // Store the entry
        self.storage.store(&entry).await?;
        
        // Log to tracing system as well
        match entry.outcome {
            AuditOutcome::Success => {
                info!("Audit: {} - {}", entry.event_type_string(), entry.summary());
            }
            AuditOutcome::Failure => {
                warn!("Audit: {} - {} (FAILED)", entry.event_type_string(), entry.summary());
            }
            AuditOutcome::Blocked => {
                error!("Audit: {} - {} (BLOCKED)", entry.event_type_string(), entry.summary());
            }
            AuditOutcome::PendingApproval => {
                info!("Audit: {} - {} (PENDING APPROVAL)", entry.event_type_string(), entry.summary());
            }
        }
        
        Ok(())
    }
    
    /// Query audit entries
    pub async fn query(
        &self,
        filters: &AuditQueryFilters,
    ) -> Result<Vec<AuditEntry>, Box<dyn std::error::Error + Send + Sync>> {
        self.storage.query(filters).await
    }
    
    /// Clean up old audit entries
    pub async fn cleanup(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let cutoff = Utc::now() - chrono::Duration::days(self.config.retention_days as i64);
        self.storage.cleanup(cutoff).await
    }
    
    /// Check if event type should be audited
    fn should_audit_event(&self, event_type: &AuditEventType) -> bool {
        self.config.events.contains(&AuditEventType::All) ||
        self.config.events.contains(event_type)
    }
    
    /// Mask sensitive data in audit entry
    fn mask_sensitive_data(&self, entry: &mut AuditEntry) {
        // Mask sensitive headers
        if let Some(ref mut request) = entry.request {
            for (name, value) in request.headers.iter_mut() {
                if self.is_sensitive_header(name) {
                    *value = "[MASKED]".to_string();
                }
            }
        }
        
        // Mask sensitive tool parameters
        if let Some(ref mut tool) = entry.tool {
            if let Some(ref mut parameters) = tool.parameters {
                for (name, value) in parameters.iter_mut() {
                    if self.is_sensitive_parameter(name) {
                        *value = serde_json::Value::String("[MASKED]".to_string());
                    }
                }
            }
        }
    }
    
    /// Check if header is sensitive
    fn is_sensitive_header(&self, name: &str) -> bool {
        let sensitive_headers = [
            "authorization",
            "cookie",
            "x-api-key",
            "x-auth-token",
            "authentication",
        ];
        
        sensitive_headers.contains(&name.to_lowercase().as_str())
    }
    
    /// Check if parameter is sensitive
    fn is_sensitive_parameter(&self, name: &str) -> bool {
        let sensitive_params = [
            "password",
            "secret",
            "token",
            "key",
            "credential",
            "auth",
        ];
        
        let name_lower = name.to_lowercase();
        sensitive_params.iter().any(|&param| name_lower.contains(param))
    }
    
    /// Limit body size in audit entry
    fn limit_body_size(&self, entry: &mut AuditEntry) {
        if let Some(ref mut request) = entry.request {
            if let Some(ref mut body) = request.body {
                if body.len() > self.config.max_body_size {
                    *body = format!("{}... [TRUNCATED]", &body[..self.config.max_body_size]);
                }
            }
        }
        
        if let Some(ref mut response) = entry.response {
            if let Some(ref mut body) = response.body {
                if body.len() > self.config.max_body_size {
                    *body = format!("{}... [TRUNCATED]", &body[..self.config.max_body_size]);
                }
            }
        }
    }
}

impl AuditEntry {
    /// Generate a unique ID for the entry
    pub fn generate_id() -> String {
        use std::time::SystemTime;
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("audit_{}", timestamp)
    }
    
    /// Get event type as string
    pub fn event_type_string(&self) -> String {
        match self.event_type {
            AuditEventType::Authentication => "AUTH".to_string(),
            AuditEventType::Authorization => "AUTHZ".to_string(),
            AuditEventType::ToolExecution => "TOOL".to_string(),
            AuditEventType::ResourceAccess => "RESOURCE".to_string(),
            AuditEventType::PromptAccess => "PROMPT".to_string(),
            AuditEventType::ConfigurationChange => "CONFIG".to_string(),
            AuditEventType::Error => "ERROR".to_string(),
            AuditEventType::SecurityViolation => "SECURITY".to_string(),
            AuditEventType::System => "SYSTEM".to_string(),
            AuditEventType::All => "ALL".to_string(),
        }
    }
    
    /// Generate a summary of the audit entry
    pub fn summary(&self) -> String {
        match (&self.request, &self.tool, &self.resource) {
            (Some(request), Some(tool), _) => {
                format!("{} {} -> {}", request.method, request.path, tool.name)
            }
            (Some(request), _, Some(resource)) => {
                format!("{} {} -> {}", request.method, request.path, resource.uri)
            }
            (Some(request), _, _) => {
                format!("{} {}", request.method, request.path)
            }
            (_, Some(tool), _) => {
                format!("Tool: {}", tool.name)
            }
            (_, _, Some(resource)) => {
                format!("Resource: {}", resource.uri)
            }
            _ => {
                format!("Event: {:?}", self.event_type)
            }
        }
    }
}

/// In-memory audit storage (for testing and small deployments)
pub struct MemoryAuditStorage {
    entries: tokio::sync::RwLock<Vec<AuditEntry>>,
    max_entries: usize,
}

impl MemoryAuditStorage {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: tokio::sync::RwLock::new(Vec::new()),
            max_entries,
        }
    }
}

#[async_trait::async_trait]
impl AuditStorage for MemoryAuditStorage {
    async fn store(&self, entry: &AuditEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut entries = self.entries.write().await;
        entries.push(entry.clone());
        
        // Keep only the most recent entries
        if entries.len() > self.max_entries {
            let entries_len = entries.len();
            entries.drain(0..entries_len - self.max_entries);
        }
        
        Ok(())
    }
    
    async fn query(
        &self,
        filters: &AuditQueryFilters,
    ) -> Result<Vec<AuditEntry>, Box<dyn std::error::Error + Send + Sync>> {
        let entries = self.entries.read().await;
        let mut filtered: Vec<AuditEntry> = entries
            .iter()
            .filter(|entry| self.matches_filters(entry, filters))
            .cloned()
            .collect();
        
        // Sort by timestamp (newest first)
        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        // Apply pagination
        if let Some(offset) = filters.offset {
            if offset < filtered.len() {
                filtered = filtered[offset..].to_vec();
            } else {
                filtered.clear();
            }
        }
        
        if let Some(limit) = filters.limit {
            filtered.truncate(limit);
        }
        
        Ok(filtered)
    }
    
    async fn cleanup(&self, _older_than: DateTime<Utc>) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let mut entries = self.entries.write().await;
        let original_len = entries.len();
        entries.retain(|entry| entry.timestamp > _older_than);
        Ok((original_len - entries.len()) as u64)
    }
}

impl MemoryAuditStorage {
    fn matches_filters(&self, entry: &AuditEntry, filters: &AuditQueryFilters) -> bool {
        if let Some(start_time) = filters.start_time {
            if entry.timestamp < start_time {
                return false;
            }
        }
        
        if let Some(end_time) = filters.end_time {
            if entry.timestamp > end_time {
                return false;
            }
        }
        
        if let Some(ref event_types) = filters.event_types {
            if !event_types.contains(&entry.event_type) {
                return false;
            }
        }
        
        if let Some(ref user_id) = filters.user_id {
            if let Some(ref user) = entry.user {
                if user.id.as_ref() != Some(user_id) {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        if let Some(ref tool_name) = filters.tool_name {
            if let Some(ref tool) = entry.tool {
                if &tool.name != tool_name {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        if let Some(ref outcome) = filters.outcome {
            if &entry.outcome != outcome {
                return false;
            }
        }
        
        true
    }
}

/// File-based audit storage
pub struct FileAuditStorage {
    directory: String,
    rotation: FileRotationConfig,
}

impl FileAuditStorage {
    pub async fn new(
        directory: String,
        rotation: FileRotationConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Create directory if it doesn't exist
        tokio::fs::create_dir_all(&directory).await?;
        
        Ok(Self {
            directory,
            rotation,
        })
    }
}

#[async_trait::async_trait]
impl AuditStorage for FileAuditStorage {
    async fn store(&self, entry: &AuditEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let filename = format!("{}/audit_{}.jsonl", 
            self.directory, 
            entry.timestamp.format("%Y%m%d")
        );
        
        let json_line = format!("{}\n", serde_json::to_string(entry)?);
        
        // Append to file
        tokio::fs::write(&filename, json_line).await?;
        
        // Check if rotation is needed
        self.check_rotation(&filename).await?;
        
        Ok(())
    }
    
    async fn query(
        &self,
        _filters: &AuditQueryFilters,
    ) -> Result<Vec<AuditEntry>, Box<dyn std::error::Error + Send + Sync>> {
        // File-based querying would be implemented here
        // For now, return empty list
        Ok(Vec::new())
    }
    
    async fn cleanup(&self, _older_than: DateTime<Utc>) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        // File cleanup would be implemented here
        // For now, return 0
        Ok(0)
    }
}

impl FileAuditStorage {
    async fn check_rotation(&self, filename: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let metadata = tokio::fs::metadata(filename).await?;
        
        if metadata.len() > self.rotation.max_file_size {
            // Rotation logic would be implemented here
            debug!("File rotation needed for {}", filename);
        }
        
        Ok(())
    }
}