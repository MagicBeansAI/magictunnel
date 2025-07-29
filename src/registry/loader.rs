//! Registry loader for discovering and loading capability files

use crate::config::RegistryConfig;
use crate::error::{ProxyError, Result};
use crate::registry::types::*;
use std::path::Path;
use tracing::{debug, info, warn};

/// Registry loader that discovers and loads capability files
pub struct RegistryLoader {
    config: RegistryConfig,
}

impl RegistryLoader {
    /// Create a new registry loader
    pub fn new(config: RegistryConfig) -> Self {
        Self { config }
    }

    /// Load all capability files from configured paths
    pub async fn load_all(&self) -> Result<Vec<CapabilityFile>> {
        info!("Loading capability files from {} paths", self.config.paths.len());
        
        let mut all_files = Vec::new();
        
        for path in &self.config.paths {
            debug!("Loading capabilities from path: {}", path);
            
            match self.load_from_path(path).await {
                Ok(mut files) => {
                    info!("Loaded {} capability files from {}", files.len(), path);
                    all_files.append(&mut files);
                }
                Err(e) => {
                    warn!("Failed to load capabilities from {}: {}", path, e);
                    if self.config.validation.strict {
                        return Err(e);
                    }
                }
            }
        }
        
        info!("Total capability files loaded: {}", all_files.len());
        Ok(all_files)
    }

    /// Load capability files from a single path (file or directory)
    async fn load_from_path(&self, path: &str) -> Result<Vec<CapabilityFile>> {
        let path = Path::new(path);
        
        if path.is_file() {
            // Single file
            let file = self.load_file(path).await?;
            Ok(vec![file])
        } else if path.is_dir() {
            // Directory - discover YAML files
            self.discover_and_load_directory(path).await
        } else {
            // Try glob pattern
            self.load_glob_pattern(path.to_string_lossy().as_ref()).await
        }
    }

    /// Load a single capability file
    async fn load_file(&self, path: &Path) -> Result<CapabilityFile> {
        debug!("Loading capability file: {}", path.display());
        
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            ProxyError::registry(format!("Failed to read file {}: {}", path.display(), e))
        })?;

        let capability_file: CapabilityFile = serde_yaml::from_str(&content).map_err(|e| {
            ProxyError::registry(format!("Failed to parse YAML file {}: {}", path.display(), e))
        })?;

        // Validate if strict mode is enabled
        if self.config.validation.strict {
            self.validate_capability_file(&capability_file)?;
        }

        Ok(capability_file)
    }

    /// Discover and load all YAML files in a directory
    async fn discover_and_load_directory(&self, dir: &Path) -> Result<Vec<CapabilityFile>> {
        let mut files = Vec::new();
        
        let mut entries = tokio::fs::read_dir(dir).await.map_err(|e| {
            ProxyError::registry(format!("Failed to read directory {}: {}", dir.display(), e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            ProxyError::registry(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "yaml" || ext == "yml" {
                        match self.load_file(&path).await {
                            Ok(file) => files.push(file),
                            Err(e) => {
                                warn!("Failed to load {}: {}", path.display(), e);
                                if self.config.validation.strict {
                                    return Err(e);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(files)
    }

    /// Load files matching a glob pattern
    async fn load_glob_pattern(&self, pattern: &str) -> Result<Vec<CapabilityFile>> {
        debug!("Loading capabilities with glob pattern: {}", pattern);
        
        let mut files = Vec::new();
        
        for entry in glob::glob(pattern).map_err(|e| {
            ProxyError::registry(format!("Invalid glob pattern {}: {}", pattern, e))
        })? {
            match entry {
                Ok(path) => {
                    if path.is_file() {
                        match self.load_file(&path).await {
                            Ok(file) => files.push(file),
                            Err(e) => {
                                warn!("Failed to load {}: {}", path.display(), e);
                                if self.config.validation.strict {
                                    return Err(e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Glob error: {}", e);
                    if self.config.validation.strict {
                        return Err(ProxyError::registry(format!("Glob error: {}", e)));
                    }
                }
            }
        }
        
        Ok(files)
    }

    /// Validate a capability file
    fn validate_capability_file(&self, file: &CapabilityFile) -> Result<()> {
        // Basic validation
        if file.tools.is_empty() {
            return Err(ProxyError::validation("Capability file must contain at least one tool"));
        }

        for tool_def in &file.tools {
            // Validate tool name
            if tool_def.name.is_empty() {
                return Err(ProxyError::validation("Tool name cannot be empty"));
            }

            // Validate tool description
            if tool_def.description.is_empty() {
                return Err(ProxyError::validation("Tool description cannot be empty"));
            }

            // Validate routing config
            if tool_def.routing.r#type.is_empty() {
                return Err(ProxyError::validation("Routing type cannot be empty"));
            }

            // TODO: Add JSON Schema validation for input_schema
        }

        Ok(())
    }
}
