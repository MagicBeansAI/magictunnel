//! Enhanced Tool Description Storage Service
//!
//! This module provides persistent storage for enhanced tool descriptions generated
//! by the sampling/elicitation pipeline, with versioning and cleanup capabilities.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{debug, info, warn, error};
use uuid::Uuid;

use crate::error::{ProxyError, Result};
use crate::discovery::types::EnhancedToolDefinition;

/// Configuration for enhancement storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementStorageConfig {
    /// Base directory for storing enhanced tool descriptions
    pub storage_dir: String,
    /// Maximum storage size in MB
    pub max_storage_mb: Option<u64>,
    /// Content cleanup policy
    pub cleanup_policy: EnhancementCleanupPolicy,
    /// Whether to enable versioning
    pub enable_versioning: bool,
    /// Whether to auto-load on startup
    pub auto_load_on_startup: bool,
}

/// Enhancement cleanup policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementCleanupPolicy {
    /// Maximum age in days before cleanup
    pub max_age_days: u64,
    /// Maximum number of versions to keep per tool
    pub max_versions_per_tool: u32,
    /// Whether to cleanup on startup
    pub cleanup_on_startup: bool,
}

/// Stored enhanced tool with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEnhancedTool {
    /// Enhanced tool definition
    pub enhanced_tool: EnhancedToolDefinition,
    /// Storage metadata
    pub metadata: EnhancementStorageMetadata,
}

/// Storage metadata for tracking enhanced tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementStorageMetadata {
    /// Storage ID
    pub id: String,
    /// Tool name this enhancement belongs to
    pub tool_name: String,
    /// Storage timestamp
    pub stored_at: String,
    /// Enhancement version
    pub version: String,
    /// File path relative to storage directory
    pub file_path: String,
    /// Original base tool definition hash (for change detection)
    pub base_tool_hash: String,
    /// Enhancement generation metadata
    pub generation_metadata: Option<crate::discovery::types::EnhancementGenerationMetadata>,
}

/// Enhancement storage service with versioning and persistence
pub struct EnhancementStorageService {
    config: EnhancementStorageConfig,
    storage_dir: PathBuf,
}

impl EnhancementStorageService {
    /// Create a new enhancement storage service
    pub fn new(config: EnhancementStorageConfig) -> Result<Self> {
        let storage_dir = PathBuf::from(&config.storage_dir);
        
        Ok(Self {
            config,
            storage_dir,
        })
    }

    /// Initialize storage directories and perform cleanup if configured
    pub async fn initialize(&self) -> Result<()> {
        self.ensure_storage_directories().await?;
        
        if self.config.cleanup_policy.cleanup_on_startup {
            self.cleanup_old_enhancements().await?;
        }
        
        info!("Enhancement storage service initialized at: {}", self.storage_dir.display());
        Ok(())
    }

    /// Store an enhanced tool description with versioning
    pub async fn store_enhanced_tool(
        &self,
        tool_name: &str,
        enhanced_tool: EnhancedToolDefinition,
        base_tool_hash: String,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let version = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let file_name = format!("{}_{}_{}_{}.json", tool_name, version, &id[..8], "enhanced");
        let file_path = self.storage_dir.join("enhancements").join(&file_name);

        let storage_metadata = EnhancementStorageMetadata {
            id: id.clone(),
            tool_name: tool_name.to_string(),
            stored_at: chrono::Utc::now().to_rfc3339(),
            version: version.clone(),
            file_path: format!("enhancements/{}", file_name),
            base_tool_hash,
            generation_metadata: enhanced_tool.enhancement_metadata.clone(),
        };

        let stored_enhanced = StoredEnhancedTool {
            enhanced_tool: enhanced_tool.clone(),
            metadata: storage_metadata,
        };

        // Write to file
        let json_content = serde_json::to_string_pretty(&stored_enhanced)
            .map_err(|e| ProxyError::config(format!("Failed to serialize enhanced tool: {}", e)))?;
        
        fs::write(&file_path, json_content).await
            .map_err(|e| ProxyError::config(format!("Failed to write enhanced tool file '{}': {}", file_path.display(), e)))?;

        debug!("Stored enhanced tool '{}' version '{}': {}", tool_name, version, file_path.display());

        // Cleanup old versions if versioning is enabled
        if self.config.enable_versioning {
            self.cleanup_old_versions_for_tool(tool_name).await?;
        }

        Ok(())
    }

    /// Load enhanced tool descriptions from storage
    pub async fn load_enhanced_tool(&self, tool_name: &str) -> Result<Option<EnhancedToolDefinition>> {
        // Find the latest version for this tool
        let latest_file = self.find_latest_version_for_tool(tool_name).await?;
        
        let file_path = match latest_file {
            Some(path) => path,
            None => {
                debug!("No stored enhanced tool found for: {}", tool_name);
                return Ok(None);
            }
        };

        let json_content = fs::read_to_string(&file_path).await
            .map_err(|e| ProxyError::config(format!("Failed to read enhanced tool file '{}': {}", file_path.display(), e)))?;
        
        let stored_enhanced: StoredEnhancedTool = serde_json::from_str(&json_content)
            .map_err(|e| ProxyError::config(format!("Failed to parse enhanced tool file '{}': {}", file_path.display(), e)))?;

        debug!("Loaded enhanced tool '{}' from storage", tool_name);
        Ok(Some(stored_enhanced.enhanced_tool))
    }

    /// Load all enhanced tool descriptions from storage
    pub async fn load_all_enhanced_tools(&self) -> Result<HashMap<String, EnhancedToolDefinition>> {
        let mut enhanced_tools = HashMap::new();
        let enhancements_dir = self.storage_dir.join("enhancements");
        
        if !enhancements_dir.exists() {
            debug!("Enhancements directory does not exist, returning empty map");
            return Ok(enhanced_tools);
        }

        let mut entries = fs::read_dir(&enhancements_dir).await
            .map_err(|e| ProxyError::config(format!("Failed to read enhancements directory: {}", e)))?;
        
        let mut tool_files: HashMap<String, (PathBuf, String)> = HashMap::new(); // tool_name -> (latest_file, version)
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| ProxyError::config(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    // Parse filename: toolname_version_id_enhanced.json
                    let parts: Vec<&str> = file_name.splitn(4, '_').collect();
                    if parts.len() >= 4 && parts[3].starts_with("enhanced") {
                        let tool_name = parts[0].to_string();
                        let version = parts[1].to_string();
                        
                        // Keep only the latest version for each tool
                        match tool_files.get(&tool_name) {
                            Some((_, existing_version)) => {
                                if version > *existing_version {
                                    tool_files.insert(tool_name, (path, version));
                                }
                            }
                            None => {
                                tool_files.insert(tool_name, (path, version));
                            }
                        }
                    }
                }
            }
        }

        // Load the latest version of each tool
        for (tool_name, (file_path, _)) in tool_files {
            match self.load_stored_enhanced_tool(&file_path).await {
                Ok(stored_enhanced) => {
                    enhanced_tools.insert(tool_name, stored_enhanced.enhanced_tool);
                }
                Err(e) => {
                    warn!("Failed to load enhanced tool from '{}': {}", file_path.display(), e);
                }
            }
        }

        info!("Loaded {} enhanced tools from storage", enhanced_tools.len());
        Ok(enhanced_tools)
    }

    /// Check if an enhanced tool exists and if the base tool has changed
    pub async fn is_enhancement_current(&self, tool_name: &str, base_tool_hash: &str) -> Result<bool> {
        let latest_file = self.find_latest_version_for_tool(tool_name).await?;
        
        let file_path = match latest_file {
            Some(path) => path,
            None => return Ok(false), // No enhancement exists
        };

        let stored_enhanced = self.load_stored_enhanced_tool(&file_path).await?;
        
        // Check if the base tool hash matches (i.e., base tool hasn't changed)
        Ok(stored_enhanced.metadata.base_tool_hash == base_tool_hash)
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self) -> Result<EnhancementStorageStats> {
        let enhancements_dir = self.storage_dir.join("enhancements");
        let mut total_files = 0;
        let mut total_size_bytes = 0;
        let mut tools_count = std::collections::HashSet::new();
        let mut oldest_file: Option<chrono::DateTime<chrono::Utc>> = None;
        let mut newest_file: Option<chrono::DateTime<chrono::Utc>> = None;

        if enhancements_dir.exists() {
            let mut entries = fs::read_dir(&enhancements_dir).await
                .map_err(|e| ProxyError::config(format!("Failed to read enhancements directory: {}", e)))?;
            
            while let Some(entry) = entries.next_entry().await
                .map_err(|e| ProxyError::config(format!("Failed to read directory entry: {}", e)))? {
                
                let path = entry.path();
                if path.is_file() {
                    total_files += 1;
                    
                    if let Ok(metadata) = fs::metadata(&path).await {
                        total_size_bytes += metadata.len();
                        
                        if let Ok(modified) = metadata.modified() {
                            let modified_datetime: chrono::DateTime<chrono::Utc> = modified.into();
                            
                            match oldest_file {
                                Some(oldest) => {
                                    if modified_datetime < oldest {
                                        oldest_file = Some(modified_datetime);
                                    }
                                }
                                None => oldest_file = Some(modified_datetime),
                            }
                            
                            match newest_file {
                                Some(newest) => {
                                    if modified_datetime > newest {
                                        newest_file = Some(modified_datetime);
                                    }
                                }
                                None => newest_file = Some(modified_datetime),
                            }
                        }
                    }
                    
                    // Extract tool name from filename
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        let parts: Vec<&str> = file_name.splitn(2, '_').collect();
                        if parts.len() >= 1 {
                            tools_count.insert(parts[0].to_string());
                        }
                    }
                }
            }
        }

        Ok(EnhancementStorageStats {
            total_files,
            total_size_bytes,
            total_size_mb: (total_size_bytes as f64) / (1024.0 * 1024.0),
            tools_with_enhancements: tools_count.len(),
            oldest_file,
            newest_file,
        })
    }

    /// Cleanup old enhancement files based on policy
    pub async fn cleanup_old_enhancements(&self) -> Result<()> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(self.config.cleanup_policy.max_age_days as i64);
        let mut cleanup_count = 0;

        let enhancements_dir = self.storage_dir.join("enhancements");
        if enhancements_dir.exists() {
            cleanup_count += self.cleanup_directory(&enhancements_dir, &cutoff_date).await?;
        }

        if cleanup_count > 0 {
            info!("Cleaned up {} old enhancement files", cleanup_count);
        }

        Ok(())
    }

    /// Find the latest version file for a specific tool
    async fn find_latest_version_for_tool(&self, tool_name: &str) -> Result<Option<PathBuf>> {
        let enhancements_dir = self.storage_dir.join("enhancements");
        
        if !enhancements_dir.exists() {
            return Ok(None);
        }

        let mut entries = fs::read_dir(&enhancements_dir).await
            .map_err(|e| ProxyError::config(format!("Failed to read enhancements directory: {}", e)))?;
        
        let mut latest_file: Option<(PathBuf, String)> = None; // (path, version)
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| ProxyError::config(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    // Parse filename: toolname_version_id_enhanced.json
                    if file_name.starts_with(&format!("{}_", tool_name)) && file_name.ends_with("_enhanced.json") {
                        let parts: Vec<&str> = file_name.splitn(4, '_').collect();
                        if parts.len() >= 2 {
                            let version = parts[1].to_string();
                            
                            match &latest_file {
                                Some((_, existing_version)) => {
                                    if version > *existing_version {
                                        latest_file = Some((path, version));
                                    }
                                }
                                None => {
                                    latest_file = Some((path, version));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(latest_file.map(|(path, _)| path))
    }

    /// Cleanup old versions for a specific tool, keeping only the configured number
    async fn cleanup_old_versions_for_tool(&self, tool_name: &str) -> Result<()> {
        let enhancements_dir = self.storage_dir.join("enhancements");
        
        if !enhancements_dir.exists() {
            return Ok(());
        }

        let mut entries = fs::read_dir(&enhancements_dir).await
            .map_err(|e| ProxyError::config(format!("Failed to read enhancements directory: {}", e)))?;
        
        let mut tool_files: Vec<(PathBuf, String)> = Vec::new(); // (path, version)
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| ProxyError::config(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with(&format!("{}_", tool_name)) && file_name.ends_with("_enhanced.json") {
                        let parts: Vec<&str> = file_name.splitn(4, '_').collect();
                        if parts.len() >= 2 {
                            let version = parts[1].to_string();
                            tool_files.push((path, version));
                        }
                    }
                }
            }
        }

        // Sort by version (descending) to keep the newest ones
        tool_files.sort_by(|a, b| b.1.cmp(&a.1));

        // Remove old versions beyond the limit
        let max_versions = self.config.cleanup_policy.max_versions_per_tool as usize;
        if tool_files.len() > max_versions {
            for (old_file_path, version) in tool_files.iter().skip(max_versions) {
                if let Err(e) = fs::remove_file(old_file_path).await {
                    warn!("Failed to remove old version '{}' for tool '{}': {}", version, tool_name, e);
                } else {
                    debug!("Removed old version '{}' for tool '{}'", version, tool_name);
                }
            }
        }

        Ok(())
    }

    /// Load a stored enhanced tool from file
    async fn load_stored_enhanced_tool(&self, path: &Path) -> Result<StoredEnhancedTool> {
        let json_content = fs::read_to_string(path).await
            .map_err(|e| ProxyError::config(format!("Failed to read enhanced tool file '{}': {}", path.display(), e)))?;
        
        serde_json::from_str(&json_content)
            .map_err(|e| ProxyError::config(format!("Failed to parse enhanced tool file '{}': {}", path.display(), e)))
    }

    /// Ensure storage directories exist
    async fn ensure_storage_directories(&self) -> Result<()> {
        let enhancements_dir = self.storage_dir.join("enhancements");

        fs::create_dir_all(&enhancements_dir).await
            .map_err(|e| ProxyError::config(format!("Failed to create enhancements directory '{}': {}", enhancements_dir.display(), e)))?;

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
                                debug!("Removed old enhancement file: {}", path.display());
                            }
                        }
                    }
                }
            }
        }

        Ok(cleanup_count)
    }
}

/// Storage statistics for enhanced tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementStorageStats {
    /// Total number of stored files
    pub total_files: usize,
    /// Total storage size in bytes
    pub total_size_bytes: u64,
    /// Total storage size in MB
    pub total_size_mb: f64,
    /// Number of tools with stored enhancements
    pub tools_with_enhancements: usize,
    /// Oldest file timestamp
    pub oldest_file: Option<chrono::DateTime<chrono::Utc>>,
    /// Newest file timestamp
    pub newest_file: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for EnhancementStorageConfig {
    fn default() -> Self {
        Self {
            storage_dir: "./storage/enhanced_tools".to_string(),
            max_storage_mb: Some(512), // 512MB default
            cleanup_policy: EnhancementCleanupPolicy {
                max_age_days: 90, // 3 months
                max_versions_per_tool: 5,
                cleanup_on_startup: true,
            },
            enable_versioning: true,
            auto_load_on_startup: true,
        }
    }
}