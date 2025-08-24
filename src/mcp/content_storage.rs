//! Content Storage and Resolution Service
//!
//! This module provides persistent storage for generated prompts and resources,
//! and resolution services to convert references to full content for MCP clients.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::error::{ProxyError, Result};
use crate::registry::types::{PromptReference, ResourceReference, GenerationReferenceMetadata};
use crate::mcp::types::{PromptTemplate, Resource, ResourceContent};

/// Configuration for content storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentStorageConfig {
    /// Base directory for storing generated content
    pub storage_dir: String,
    /// Maximum storage size in MB
    pub max_storage_mb: Option<u64>,
    /// Content cleanup policy
    pub cleanup_policy: CleanupPolicy,
    /// Whether to enable versioning
    pub enable_versioning: bool,
}

/// Content cleanup policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPolicy {
    /// Maximum age in days before cleanup
    pub max_age_days: u64,
    /// Maximum number of versions to keep
    pub max_versions: u32,
    /// Whether to cleanup on startup
    pub cleanup_on_startup: bool,
}

/// Stored prompt content with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPrompt {
    /// Prompt template
    pub template: PromptTemplate,
    /// Full prompt content
    pub content: String,
    /// Storage metadata
    pub metadata: StorageMetadata,
}

/// Stored resource content with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredResource {
    /// Resource definition
    pub resource: Resource,
    /// Resource content
    pub content: ResourceContent,
    /// Storage metadata
    pub metadata: StorageMetadata,
}

/// Storage metadata for tracking content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetadata {
    /// Storage ID
    pub id: String,
    /// Tool name this content belongs to
    pub tool_name: String,
    /// Content type (prompt/resource)
    pub content_type: String,
    /// Content subtype (e.g., usage, documentation)
    pub content_subtype: String,
    /// Generation metadata
    pub generation_metadata: Option<GenerationReferenceMetadata>,
    /// Storage timestamp
    pub stored_at: String,
    /// Content version
    pub version: String,
    /// File path relative to storage directory
    pub file_path: String,
}

/// Content storage and resolution service
pub struct ContentStorageService {
    config: ContentStorageConfig,
    storage_dir: PathBuf,
}

impl ContentStorageService {
    /// Create a new content storage service
    pub fn new(config: ContentStorageConfig) -> Result<Self> {
        let storage_dir = PathBuf::from(&config.storage_dir);
        
        Ok(Self {
            config,
            storage_dir,
        })
    }

    /// Initialize storage directories
    pub async fn initialize(&self) -> Result<()> {
        self.ensure_storage_directories().await?;
        
        if self.config.cleanup_policy.cleanup_on_startup {
            self.cleanup_old_content().await?;
        }
        
        info!("Content storage service initialized at: {}", self.storage_dir.display());
        Ok(())
    }

    /// Store a generated prompt and return reference
    pub async fn store_prompt(
        &self,
        tool_name: &str,
        prompt_type: &str,
        template: PromptTemplate,
        content: String,
        generation_metadata: Option<GenerationReferenceMetadata>,
    ) -> Result<PromptReference> {
        let id = Uuid::new_v4().to_string();
        let version = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let file_name = format!("{}_{}_{}_{}.json", tool_name, prompt_type, version, &id[..8]);
        let file_path = self.storage_dir.join("prompts").join(&file_name);

        let storage_metadata = StorageMetadata {
            id: id.clone(),
            tool_name: tool_name.to_string(),
            content_type: "prompt".to_string(),
            content_subtype: prompt_type.to_string(),
            generation_metadata: generation_metadata.clone(),
            stored_at: chrono::Utc::now().to_rfc3339(),
            version: version.clone(),
            file_path: format!("prompts/{}", file_name),
        };

        let stored_prompt = StoredPrompt {
            template: template.clone(),
            content,
            metadata: storage_metadata,
        };

        // Write to file
        let json_content = serde_json::to_string_pretty(&stored_prompt)
            .map_err(|e| ProxyError::config(format!("Failed to serialize prompt: {}", e)))?;
        
        fs::write(&file_path, json_content).await
            .map_err(|e| ProxyError::config(format!("Failed to write prompt file '{}': {}", file_path.display(), e)))?;

        debug!("Stored prompt for tool '{}' type '{}': {}", tool_name, prompt_type, file_path.display());

        Ok(PromptReference {
            name: format!("{}_{}", tool_name, prompt_type),
            prompt_type: prompt_type.to_string(),
            description: template.description.clone(),
            storage_path: Some(format!("prompts/{}", file_name)),
            generation_metadata,
        })
    }

    /// Store a generated resource and return reference
    pub async fn store_resource(
        &self,
        tool_name: &str,
        resource_type: &str,
        resource: Resource,
        content: ResourceContent,
        generation_metadata: Option<GenerationReferenceMetadata>,
    ) -> Result<ResourceReference> {
        let id = Uuid::new_v4().to_string();
        let version = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let file_name = format!("{}_{}_{}_{}.json", tool_name, resource_type, version, &id[..8]);
        let file_path = self.storage_dir.join("resources").join(&file_name);

        let storage_metadata = StorageMetadata {
            id: id.clone(),
            tool_name: tool_name.to_string(),
            content_type: "resource".to_string(),
            content_subtype: resource_type.to_string(),
            generation_metadata: generation_metadata.clone(),
            stored_at: chrono::Utc::now().to_rfc3339(),
            version: version.clone(),
            file_path: format!("resources/{}", file_name),
        };

        let stored_resource = StoredResource {
            resource: resource.clone(),
            content,
            metadata: storage_metadata,
        };

        // Write to file
        let json_content = serde_json::to_string_pretty(&stored_resource)
            .map_err(|e| ProxyError::config(format!("Failed to serialize resource: {}", e)))?;
        
        fs::write(&file_path, json_content).await
            .map_err(|e| ProxyError::config(format!("Failed to write resource file '{}': {}", file_path.display(), e)))?;

        debug!("Stored resource for tool '{}' type '{}': {}", tool_name, resource_type, file_path.display());

        Ok(ResourceReference {
            name: format!("{}_{}", tool_name, resource_type),
            resource_type: resource_type.to_string(),
            uri: resource.uri.clone(),
            mime_type: resource.mime_type.clone(),
            description: resource.description.clone(),
            storage_path: Some(format!("resources/{}", file_name)),
            generation_metadata,
        })
    }

    /// Resolve a prompt reference to full content
    pub async fn resolve_prompt(&self, prompt_ref: &PromptReference) -> Result<(PromptTemplate, String)> {
        let storage_path = prompt_ref.storage_path.as_ref()
            .ok_or_else(|| ProxyError::validation("Prompt reference missing storage path".to_string()))?;
        
        let file_path = self.storage_dir.join(storage_path);
        
        if !file_path.exists() {
            return Err(ProxyError::config(format!("Prompt storage file not found: {}", file_path.display())));
        }

        let json_content = fs::read_to_string(&file_path).await
            .map_err(|e| ProxyError::config(format!("Failed to read prompt file '{}': {}", file_path.display(), e)))?;
        
        let stored_prompt: StoredPrompt = serde_json::from_str(&json_content)
            .map_err(|e| ProxyError::config(format!("Failed to parse prompt file '{}': {}", file_path.display(), e)))?;

        debug!("Resolved prompt '{}' from storage", prompt_ref.name);
        Ok((stored_prompt.template, stored_prompt.content))
    }

    /// Resolve a resource reference to full content
    pub async fn resolve_resource(&self, resource_ref: &ResourceReference) -> Result<(Resource, ResourceContent)> {
        let storage_path = resource_ref.storage_path.as_ref()
            .ok_or_else(|| ProxyError::validation("Resource reference missing storage path".to_string()))?;
        
        let file_path = self.storage_dir.join(storage_path);
        
        if !file_path.exists() {
            return Err(ProxyError::config(format!("Resource storage file not found: {}", file_path.display())));
        }

        let json_content = fs::read_to_string(&file_path).await
            .map_err(|e| ProxyError::config(format!("Failed to read resource file '{}': {}", file_path.display(), e)))?;
        
        let stored_resource: StoredResource = serde_json::from_str(&json_content)
            .map_err(|e| ProxyError::config(format!("Failed to parse resource file '{}': {}", file_path.display(), e)))?;

        debug!("Resolved resource '{}' from storage", resource_ref.name);
        Ok((stored_resource.resource, stored_resource.content))
    }

    /// List all stored content for a tool
    pub async fn list_tool_content(&self, tool_name: &str) -> Result<(Vec<PromptReference>, Vec<ResourceReference>)> {
        let mut prompts = Vec::new();
        let mut resources = Vec::new();

        // Search prompt directory
        let prompts_dir = self.storage_dir.join("prompts");
        if prompts_dir.exists() {
            let mut entries = fs::read_dir(&prompts_dir).await
                .map_err(|e| ProxyError::config(format!("Failed to read prompts directory: {}", e)))?;
            
            while let Some(entry) = entries.next_entry().await
                .map_err(|e| ProxyError::config(format!("Failed to read directory entry: {}", e)))? {
                
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name.starts_with(&format!("{}_", tool_name)) {
                            if let Ok(stored) = self.load_stored_prompt(&path).await {
                                if let Ok(prompt_ref) = self.stored_prompt_to_reference(&stored) {
                                    prompts.push(prompt_ref);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Search resource directory
        let resources_dir = self.storage_dir.join("resources");
        if resources_dir.exists() {
            let mut entries = fs::read_dir(&resources_dir).await
                .map_err(|e| ProxyError::config(format!("Failed to read resources directory: {}", e)))?;
            
            while let Some(entry) = entries.next_entry().await
                .map_err(|e| ProxyError::config(format!("Failed to read directory entry: {}", e)))? {
                
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name.starts_with(&format!("{}_", tool_name)) {
                            if let Ok(stored) = self.load_stored_resource(&path).await {
                                if let Ok(resource_ref) = self.stored_resource_to_reference(&stored) {
                                    resources.push(resource_ref);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((prompts, resources))
    }

    /// List all stored content across all tools
    pub async fn list_all_content(&self) -> Result<(Vec<PromptReference>, Vec<ResourceReference>)> {
        let mut prompts = Vec::new();
        let mut resources = Vec::new();

        // Search prompt directory
        let prompts_dir = self.storage_dir.join("prompts");
        if prompts_dir.exists() {
            if let Ok(entries) = tokio::fs::read_dir(&prompts_dir).await {
                let mut entries = entries;
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Ok(file_type) = entry.file_type().await {
                        if file_type.is_file() {
                            if let Some(file_name) = entry.file_name().to_str() {
                                if file_name.ends_with(".json") {
                                    if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                                        if let Ok(prompt_ref) = serde_json::from_str::<PromptReference>(&content) {
                                            prompts.push(prompt_ref);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Search resource directory  
        let resources_dir = self.storage_dir.join("resources");
        if resources_dir.exists() {
            if let Ok(entries) = tokio::fs::read_dir(&resources_dir).await {
                let mut entries = entries;
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Ok(file_type) = entry.file_type().await {
                        if file_type.is_file() {
                            if let Some(file_name) = entry.file_name().to_str() {
                                if file_name.ends_with(".json") {
                                    if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                                        if let Ok(resource_ref) = serde_json::from_str::<ResourceReference>(&content) {
                                            resources.push(resource_ref);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((prompts, resources))
    }

    /// Cleanup old content based on policy
    pub async fn cleanup_old_content(&self) -> Result<()> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(self.config.cleanup_policy.max_age_days as i64);
        let mut cleanup_count = 0;

        // Cleanup prompts
        let prompts_dir = self.storage_dir.join("prompts");
        if prompts_dir.exists() {
            cleanup_count += self.cleanup_directory(&prompts_dir, &cutoff_date).await?;
        }

        // Cleanup resources
        let resources_dir = self.storage_dir.join("resources");
        if resources_dir.exists() {
            cleanup_count += self.cleanup_directory(&resources_dir, &cutoff_date).await?;
        }

        if cleanup_count > 0 {
            info!("Cleaned up {} old content files", cleanup_count);
        }

        Ok(())
    }

    /// Ensure storage directories exist
    async fn ensure_storage_directories(&self) -> Result<()> {
        let prompts_dir = self.storage_dir.join("prompts");
        let resources_dir = self.storage_dir.join("resources");

        fs::create_dir_all(&prompts_dir).await
            .map_err(|e| ProxyError::config(format!("Failed to create prompts directory '{}': {}", prompts_dir.display(), e)))?;
        
        fs::create_dir_all(&resources_dir).await
            .map_err(|e| ProxyError::config(format!("Failed to create resources directory '{}': {}", resources_dir.display(), e)))?;

        Ok(())
    }

    /// Cleanup files in a directory based on age
    async fn cleanup_directory(&self, dir: &Path, cutoff_date: &chrono::DateTime<chrono::Utc>) -> Result<usize> {
        let mut cleanup_count = 0;
        let mut entries = fs::read_dir(dir).await
            .map_err(|e| ProxyError::config(format!("Failed to read directory '{}': {}", dir.display(), e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| ProxyError::config(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.is_file() {
                if let Ok(metadata) = fs::metadata(&path).await {
                    if let Ok(modified) = metadata.modified() {
                        let modified_datetime: chrono::DateTime<chrono::Utc> = modified.into();
                        if modified_datetime < *cutoff_date {
                            if let Err(e) = fs::remove_file(&path).await {
                                warn!("Failed to remove old file '{}': {}", path.display(), e);
                            } else {
                                cleanup_count += 1;
                                debug!("Removed old content file: {}", path.display());
                            }
                        }
                    }
                }
            }
        }

        Ok(cleanup_count)
    }

    /// Load a stored prompt from file
    async fn load_stored_prompt(&self, path: &Path) -> Result<StoredPrompt> {
        let json_content = fs::read_to_string(path).await
            .map_err(|e| ProxyError::config(format!("Failed to read prompt file '{}': {}", path.display(), e)))?;
        
        serde_json::from_str(&json_content)
            .map_err(|e| ProxyError::config(format!("Failed to parse prompt file '{}': {}", path.display(), e)))
    }

    /// Load a stored resource from file
    async fn load_stored_resource(&self, path: &Path) -> Result<StoredResource> {
        let json_content = fs::read_to_string(path).await
            .map_err(|e| ProxyError::config(format!("Failed to read resource file '{}': {}", path.display(), e)))?;
        
        serde_json::from_str(&json_content)
            .map_err(|e| ProxyError::config(format!("Failed to parse resource file '{}': {}", path.display(), e)))
    }

    /// Convert stored prompt to reference
    fn stored_prompt_to_reference(&self, stored: &StoredPrompt) -> Result<PromptReference> {
        Ok(PromptReference {
            name: format!("{}_{}", stored.metadata.tool_name, stored.metadata.content_subtype),
            prompt_type: stored.metadata.content_subtype.clone(),
            description: stored.template.description.clone(),
            storage_path: Some(stored.metadata.file_path.clone()),
            generation_metadata: stored.metadata.generation_metadata.clone(),
        })
    }

    /// Convert stored resource to reference
    fn stored_resource_to_reference(&self, stored: &StoredResource) -> Result<ResourceReference> {
        Ok(ResourceReference {
            name: format!("{}_{}", stored.metadata.tool_name, stored.metadata.content_subtype),
            resource_type: stored.metadata.content_subtype.clone(),
            uri: stored.resource.uri.clone(),
            mime_type: stored.resource.mime_type.clone(),
            description: stored.resource.description.clone(),
            storage_path: Some(stored.metadata.file_path.clone()),
            generation_metadata: stored.metadata.generation_metadata.clone(),
        })
    }
}

impl Default for ContentStorageConfig {
    fn default() -> Self {
        Self {
            storage_dir: "./storage/generated_content".to_string(),
            max_storage_mb: Some(1024), // 1GB default
            cleanup_policy: CleanupPolicy {
                max_age_days: 90, // 3 months
                max_versions: 10,
                cleanup_on_startup: true,
            },
            enable_versioning: true,
        }
    }
}