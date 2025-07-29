//! MCP Resource Management
//!
//! This module implements basic MCP resource management functionality including:
//! - Resource discovery and listing
//! - Resource content reading
//! - File-based resource providers
//! - Resource URI handling

use crate::error::{Result, ProxyError};
use crate::mcp::types::{Resource, ResourceContent, ResourceAnnotations};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Resource provider trait for different resource sources
#[async_trait::async_trait]
pub trait ResourceProvider: Send + Sync {
    /// List available resources
    async fn list_resources(&self, cursor: Option<String>) -> Result<(Vec<Resource>, Option<String>)>;
    
    /// Read resource content by URI
    async fn read_resource(&self, uri: &str) -> Result<ResourceContent>;
    
    /// Check if provider supports the given URI scheme
    fn supports_uri(&self, uri: &str) -> bool;
    
    /// Get provider name for debugging
    fn name(&self) -> &str;
}

/// File-based resource provider
pub struct FileResourceProvider {
    /// Base directory for file resources
    base_dir: PathBuf,
    /// URI prefix for this provider
    uri_prefix: String,
    /// Provider name
    name: String,
}

impl FileResourceProvider {
    /// Create a new file resource provider
    pub fn new<P: AsRef<Path>>(base_dir: P, uri_prefix: String) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        if !base_dir.exists() {
            return Err(ProxyError::validation(format!(
                "Base directory does not exist: {}",
                base_dir.display()
            )));
        }
        
        Ok(Self {
            base_dir,
            uri_prefix: uri_prefix.clone(),
            name: format!("FileProvider({})", uri_prefix),
        })
    }
    
    /// Convert file path to resource URI
    fn path_to_uri(&self, path: &Path) -> String {
        let relative_path = path.strip_prefix(&self.base_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/"); // Normalize path separators
        format!("{}/{}", self.uri_prefix.trim_end_matches('/'), relative_path)
    }
    
    /// Convert resource URI to file path
    fn uri_to_path(&self, uri: &str) -> Option<PathBuf> {
        if !uri.starts_with(&self.uri_prefix) {
            return None;
        }
        
        let relative_path = uri.strip_prefix(&self.uri_prefix)
            .unwrap_or("")
            .trim_start_matches('/');
        
        if relative_path.is_empty() {
            return None;
        }
        
        // Security check: prevent path traversal
        if relative_path.contains("..") {
            return None;
        }
        
        Some(self.base_dir.join(relative_path))
    }
    
    /// Get MIME type from file extension
    fn get_mime_type(&self, path: &Path) -> Option<String> {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("txt") => Some("text/plain".to_string()),
            Some("md") => Some("text/markdown".to_string()),
            Some("json") => Some("application/json".to_string()),
            Some("yaml") | Some("yml") => Some("application/yaml".to_string()),
            Some("toml") => Some("application/toml".to_string()),
            Some("xml") => Some("application/xml".to_string()),
            Some("html") | Some("htm") => Some("text/html".to_string()),
            Some("css") => Some("text/css".to_string()),
            Some("js") => Some("application/javascript".to_string()),
            Some("py") => Some("text/x-python".to_string()),
            Some("rs") => Some("text/x-rust".to_string()),
            Some("go") => Some("text/x-go".to_string()),
            Some("java") => Some("text/x-java".to_string()),
            Some("c") => Some("text/x-c".to_string()),
            Some("cpp") | Some("cc") | Some("cxx") => Some("text/x-c++".to_string()),
            Some("h") => Some("text/x-c-header".to_string()),
            Some("hpp") | Some("hxx") => Some("text/x-c++-header".to_string()),
            Some("sh") => Some("application/x-sh".to_string()),
            Some("bat") => Some("application/x-bat".to_string()),
            Some("ps1") => Some("application/x-powershell".to_string()),
            Some("sql") => Some("application/sql".to_string()),
            Some("log") => Some("text/plain".to_string()),
            Some("csv") => Some("text/csv".to_string()),
            Some("png") => Some("image/png".to_string()),
            Some("jpg") | Some("jpeg") => Some("image/jpeg".to_string()),
            Some("gif") => Some("image/gif".to_string()),
            Some("svg") => Some("image/svg+xml".to_string()),
            Some("pdf") => Some("application/pdf".to_string()),
            Some("zip") => Some("application/zip".to_string()),
            Some("tar") => Some("application/x-tar".to_string()),
            Some("gz") => Some("application/gzip".to_string()),
            _ => None,
        }
    }
}

#[async_trait::async_trait]
impl ResourceProvider for FileResourceProvider {
    async fn list_resources(&self, _cursor: Option<String>) -> Result<(Vec<Resource>, Option<String>)> {
        debug!("Listing resources from directory: {}", self.base_dir.display());
        
        let mut resources = Vec::new();
        let mut entries = fs::read_dir(&self.base_dir).await
            .map_err(|e| ProxyError::Io(e))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| ProxyError::Io(e))? {

            let path = entry.path();
            let metadata = entry.metadata().await
                .map_err(|e| ProxyError::Io(e))?;
            
            // Skip directories for now (basic implementation)
            if metadata.is_dir() {
                continue;
            }
            
            let uri = self.path_to_uri(&path);
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            let mime_type = self.get_mime_type(&path);
            
            let annotations = ResourceAnnotations::new()
                .with_size(metadata.len())
                .with_last_modified(
                    metadata.modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
                        .flatten()
                        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                );
            
            let resource = Resource::complete(
                uri,
                name,
                Some(format!("File: {}", path.display())),
                mime_type,
                Some(annotations),
            );
            
            resources.push(resource);
        }
        
        info!("Found {} resources in directory: {}", resources.len(), self.base_dir.display());
        Ok((resources, None)) // No pagination for basic implementation
    }
    
    async fn read_resource(&self, uri: &str) -> Result<ResourceContent> {
        debug!("Reading resource: {}", uri);
        
        let path = self.uri_to_path(uri)
            .ok_or_else(|| ProxyError::validation(format!("Invalid URI for this provider: {}", uri)))?;
        
        if !path.exists() {
            return Err(ProxyError::validation(format!("Resource not found: {}", uri)));
        }

        let metadata = fs::metadata(&path).await
            .map_err(|e| ProxyError::Io(e))?;
        
        if metadata.is_dir() {
            return Err(ProxyError::validation(format!("Cannot read directory as resource: {}", uri)));
        }
        
        let mime_type = self.get_mime_type(&path);
        
        // Determine if file should be read as text or binary
        let is_text = mime_type.as_ref()
            .map(|mt| mt.starts_with("text/") || 
                     mt == "application/json" || 
                     mt == "application/yaml" || 
                     mt == "application/toml" || 
                     mt == "application/xml" ||
                     mt == "application/javascript" ||
                     mt == "application/sql")
            .unwrap_or(false);
        
        if is_text {
            let content = fs::read_to_string(&path).await
                .map_err(|e| ProxyError::Io(e))?;

            debug!("Read text resource: {} ({} bytes)", uri, content.len());
            Ok(ResourceContent::text(uri.to_string(), content, mime_type))
        } else {
            let content = fs::read(&path).await
                .map_err(|e| ProxyError::Io(e))?;
            
            debug!("Read binary resource: {} ({} bytes)", uri, content.len());
            Ok(ResourceContent::blob(uri.to_string(), content, mime_type))
        }
    }
    
    fn supports_uri(&self, uri: &str) -> bool {
        uri.starts_with(&self.uri_prefix)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// Resource manager that coordinates multiple resource providers
pub struct ResourceManager {
    /// Registered resource providers
    providers: Arc<RwLock<Vec<Arc<dyn ResourceProvider>>>>,
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Add a resource provider
    pub async fn add_provider(&self, provider: Arc<dyn ResourceProvider>) {
        let mut providers = self.providers.write().await;
        info!("Adding resource provider: {}", provider.name());
        providers.push(provider);
    }
    
    /// List all available resources
    pub async fn list_resources(&self, cursor: Option<String>) -> Result<(Vec<Resource>, Option<String>)> {
        debug!("Listing resources from all providers");
        
        let providers = self.providers.read().await;
        let mut all_resources = Vec::new();
        
        for provider in providers.iter() {
            match provider.list_resources(cursor.clone()).await {
                Ok((resources, _)) => {
                    debug!("Provider {} returned {} resources", provider.name(), resources.len());
                    all_resources.extend(resources);
                }
                Err(e) => {
                    warn!("Provider {} failed to list resources: {}", provider.name(), e);
                    // Continue with other providers
                }
            }
        }
        
        info!("Total resources available: {}", all_resources.len());
        Ok((all_resources, None)) // No pagination for basic implementation
    }
    
    /// Read resource content by URI
    pub async fn read_resource(&self, uri: &str) -> Result<ResourceContent> {
        debug!("Reading resource: {}", uri);
        
        let providers = self.providers.read().await;
        
        for provider in providers.iter() {
            if provider.supports_uri(uri) {
                debug!("Using provider {} for URI: {}", provider.name(), uri);
                return provider.read_resource(uri).await;
            }
        }
        
        Err(ProxyError::validation(format!("No provider supports URI: {}", uri)))
    }
    
    /// Get number of registered providers
    pub async fn provider_count(&self) -> usize {
        self.providers.read().await.len()
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}
