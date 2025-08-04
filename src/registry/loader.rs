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
    pub async fn load_file(&self, path: &Path) -> Result<CapabilityFile> {
        debug!("Loading capability file: {}", path.display());
        
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            ProxyError::registry(format!("Failed to read file {}: {}", path.display(), e))
        })?;

        // Try to parse as enhanced format first, then fall back to legacy format
        let capability_file = match self.parse_enhanced_capability_file(&content) {
            Ok(capability_file) => {
                debug!("Loaded enhanced format capability file: {} (tools: {})", 
                       path.display(), capability_file.tool_count());
                capability_file
            }
            Err(enhanced_error) => {
                // Fall back to legacy format parsing
                debug!("Enhanced format parsing failed, trying legacy format for: {}: {}", path.display(), enhanced_error);
                let capability_file: CapabilityFile = serde_yaml::from_str(&content).map_err(|legacy_error| {
                    ProxyError::registry(format!(
                        "Failed to parse YAML file {} (tried both enhanced and legacy formats): {}", 
                        path.display(), legacy_error
                    ))
                })?;
                
                debug!("Loaded legacy format capability file: {} (tools: {})", 
                       path.display(), capability_file.tool_count());
                capability_file
            }
        };

        // Validate if strict mode is enabled
        if self.config.validation.strict {
            self.validate_capability_file_enhanced(&capability_file)?;
        }

        Ok(capability_file)
    }

    /// Parse enhanced capability file format and convert to legacy format
    fn parse_enhanced_capability_file(&self, content: &str) -> Result<CapabilityFile> {
        // First try to detect if this is an enhanced format file
        if !self.is_enhanced_format(content) {
            return Err(ProxyError::registry("Not an enhanced format file".to_string()));
        }

        println!("✅ Detected enhanced format, attempting to parse");
        
        // Parse as enhanced format
        let enhanced: EnhancedCapabilityFileRaw = serde_yaml::from_str(content)
            .map_err(|e| {
                println!("❌ Enhanced format YAML parsing failed: {}", e);
                ProxyError::registry(format!("Enhanced format parsing error: {}", e))
            })?;
        
        debug!("Enhanced format parsed successfully, converting to legacy");
        
        // Convert to legacy format
        self.convert_enhanced_to_legacy(enhanced)
    }

    /// Check if content appears to be enhanced format
    fn is_enhanced_format(&self, content: &str) -> bool {
        // Look for enhanced format indicators
        let has_core = content.contains("core:");
        let has_execution = content.contains("execution:");
        let has_mcp = content.contains("# MCP 2025-06-18") || content.contains("mcp_capabilities:");
        
        println!("🔍 Enhanced format detection - core: {}, execution: {}, mcp: {}", has_core, has_execution, has_mcp);
        
        has_core && has_execution && has_mcp
    }

    /// Convert enhanced format to legacy format for internal processing
    fn convert_enhanced_to_legacy(&self, enhanced: EnhancedCapabilityFileRaw) -> Result<CapabilityFile> {
        debug!("Converting enhanced format to legacy format");
        
        // Convert enhanced tools to legacy format
        let legacy_tools = enhanced.tools.into_iter().map(|enhanced_tool| {
            // Extract routing from execution.routing or use default
            let routing = if let Some(execution) = &enhanced_tool.execution {
                if let Some(execution_routing) = &execution.routing {
                    // Convert enhanced routing to legacy routing
                    RoutingConfig {
                        r#type: execution_routing.r#type.clone(),
                        config: serde_json::to_value(&execution_routing.primary).unwrap_or_else(|_| {
                            serde_json::json!({})
                        })
                    }
                } else {
                    enhanced_tool.routing.unwrap_or_else(|| {
                        RoutingConfig::new("subprocess".to_string(), serde_json::json!({}))
                    })
                }
            } else {
                enhanced_tool.routing.unwrap_or_else(|| {
                    RoutingConfig::new("subprocess".to_string(), serde_json::json!({}))
                })
            };

            // Extract visibility settings from access section or use tool-level settings
            let (hidden, enabled) = if let Some(access) = &enhanced_tool.access {
                (access.hidden.unwrap_or(false), access.enabled.unwrap_or(true))
            } else {
                (enhanced_tool.hidden.unwrap_or(false), enhanced_tool.enabled.unwrap_or(true))
            };

            ToolDefinition {
                name: enhanced_tool.name,
                description: enhanced_tool.core.description,
                input_schema: enhanced_tool.core.input_schema,
                routing,
                annotations: enhanced_tool.annotations,
                hidden,
                enabled,
                prompt_refs: enhanced_tool.prompt_refs.unwrap_or_default(),
                resource_refs: enhanced_tool.resource_refs.unwrap_or_default(),
                sampling_strategy: enhanced_tool.sampling_strategy,
                elicitation_strategy: enhanced_tool.elicitation_strategy,
            }
        }).collect();

        // Convert enhanced metadata to legacy FileMetadata
        let legacy_metadata = if let Some(enhanced_meta) = enhanced.metadata {
            Some(FileMetadata {
                name: Some(enhanced_meta.name),
                description: Some(enhanced_meta.description),
                version: Some(enhanced_meta.version),
                author: Some(enhanced_meta.author),
                tags: None,
            })
        } else {
            None
        };

        Ok(CapabilityFile {
            metadata: legacy_metadata,
            tools: legacy_tools,
            enhanced_metadata: None, // Don't preserve enhanced metadata in legacy format for now
            enhanced_tools: None, // We don't preserve enhanced tools in legacy format
        })
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

    /// Validate a capability file (enhanced validation temporarily disabled)
    fn validate_capability_file_enhanced(&self, file: &CapabilityFile) -> Result<()> {
        debug!("Validating capability file with legacy validation");
        // Use legacy validation for now
        self.validate_capability_file(file)?;
        Ok(())
    }
}
