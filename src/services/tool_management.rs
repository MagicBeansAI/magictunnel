//! Tool Management Service
//!
//! Provides runtime tool visibility and enable/disable functionality without server restart.
//! Leverages the existing CLI visibility management logic with hot-reload capabilities.

use crate::config::Config;
use crate::error::{ProxyError, Result};
use crate::registry::types::CapabilityFile;
use crate::registry::{RegistryLoader, RegistryService};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Tool state for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolState {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub hidden: bool,
    pub file_path: String,
    pub category: Option<String>,
    pub last_modified: Option<String>,
}

/// Request to update tool state
#[derive(Debug, Deserialize)]
pub struct UpdateToolStateRequest {
    pub enabled: Option<bool>,
    pub hidden: Option<bool>,
}

/// Request to bulk update multiple tools
#[derive(Debug, Deserialize)]
pub struct BulkUpdateRequest {
    pub tool_names: Vec<String>,
    pub enabled: Option<bool>,
    pub hidden: Option<bool>,
}

/// Tool management operation result
#[derive(Debug, Serialize)]
pub struct ToolManagementResult {
    pub success: bool,
    pub message: String,
    pub affected_tools: Vec<String>,
    pub total_affected: usize,
}

/// Tool management service
pub struct ToolManagementService {
    config: Arc<Config>,
    capability_cache: Arc<RwLock<HashMap<String, PathBuf>>>, // tool_name -> file_path
    registry_loader: RegistryLoader,
    registry_service: Option<Arc<RegistryService>>,
}

impl ToolManagementService {
    /// Create a new tool management service
    pub fn new(config: Arc<Config>) -> Self {
        let registry_loader = RegistryLoader::new(config.registry.clone());
        Self {
            config,
            capability_cache: Arc::new(RwLock::new(HashMap::new())),
            registry_loader,
            registry_service: None,
        }
    }
    
    /// Set registry service reference for hot-reload functionality
    pub fn with_registry_service(mut self, registry_service: Arc<RegistryService>) -> Self {
        self.registry_service = Some(registry_service);
        self
    }

    /// Initialize the service by building the tool-to-file mapping cache
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing tool management service");
        self.refresh_capability_cache().await?;
        info!("Tool management service initialized with {} tools", 
              self.capability_cache.read().await.len());
        Ok(())
    }
    
    /// Trigger registry reload after tool changes
    async fn trigger_registry_reload(&self) -> Result<()> {
        if let Some(registry_service) = &self.registry_service {
            debug!("Triggering registry reload due to tool management changes");
            if let Err(e) = registry_service.reload_registry().await {
                warn!("Failed to reload registry after tool changes: {}", e);
                return Err(e);
            }
            info!("Registry successfully reloaded after tool management changes");
        } else {
            debug!("No registry service available for hot-reload - changes will be visible after restart");
        }
        Ok(())
    }

    /// Get the state of all tools
    pub async fn get_all_tool_states(&self) -> Result<Vec<ToolState>> {
        let capability_files = self.discover_capability_files().await?;
        let mut tool_states = Vec::new();

        for file_path in capability_files {
            let capability_file = self.load_capability_file(&file_path).await?;
            
            for tool in &capability_file.tools {
                tool_states.push(ToolState {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    enabled: tool.is_enabled(),
                    hidden: tool.is_hidden(),
                    file_path: file_path.to_string_lossy().to_string(),
                    category: Some("general".to_string()), // Default category as ToolDefinition doesn't have category field
                    last_modified: None, // Could be populated from file metadata if needed
                });
            }
        }

        Ok(tool_states)
    }

    /// Get the state of a specific tool
    pub async fn get_tool_state(&self, tool_name: &str) -> Result<ToolState> {
        let file_path = self.find_tool_file(tool_name).await?;
        let capability_file = self.load_capability_file(&file_path).await?;
        
        // Try to find the tool in regular tools first
        if let Some(tool) = capability_file.get_tool(tool_name) {
            return Ok(ToolState {
                name: tool.name.clone(),
                description: tool.description.clone(),
                enabled: tool.enabled,
                hidden: tool.hidden,
                file_path: file_path.to_string_lossy().to_string(),
                category: Some("general".to_string()), // Default category as ToolDefinition doesn't have category field
                last_modified: None,
            });
        }

        // Try enhanced tools
        if let Some(enhanced_tool) = capability_file.get_enhanced_tool(tool_name) {
            return Ok(ToolState {
                name: enhanced_tool.name.clone(),
                description: enhanced_tool.core.description.clone(),
                enabled: enhanced_tool.access.enabled,
                hidden: enhanced_tool.access.hidden,
                file_path: file_path.to_string_lossy().to_string(),
                category: Some("enhanced".to_string()), // Category for enhanced tools
                last_modified: None,
            });
        }

        Err(ProxyError::registry(format!("Tool '{}' not found", tool_name)))
    }

    /// Update a single tool's state
    pub async fn update_tool_state(
        &self, 
        tool_name: &str, 
        update: UpdateToolStateRequest
    ) -> Result<ToolManagementResult> {
        let file_path = self.find_tool_file(tool_name).await?;
        let mut capability_file = self.load_capability_file(&file_path).await?;
        
        let mut modified = false;
        let mut actions = Vec::new();

        // Update enabled state
        if let Some(enabled) = update.enabled {
            capability_file.set_tool_enabled(tool_name, enabled)?;
            actions.push(format!("{} tool", if enabled { "enabled" } else { "disabled" }));
            modified = true;
        }

        // Update hidden state
        if let Some(hidden) = update.hidden {
            capability_file.set_tool_hidden(tool_name, hidden)?;
            actions.push(format!("{} tool", if hidden { "hidden" } else { "shown" }));
            modified = true;
        }

        if modified {
            self.save_capability_file(&file_path, &capability_file)?;
            info!("Updated tool '{}': {}", tool_name, actions.join(", "));
            
            // Trigger registry reload for immediate effect
            self.trigger_registry_reload().await?;
        }

        Ok(ToolManagementResult {
            success: true,
            message: if modified {
                format!("Successfully {} for tool '{}'", actions.join(" and "), tool_name)
            } else {
                "No changes requested".to_string()
            },
            affected_tools: if modified { vec![tool_name.to_string()] } else { vec![] },
            total_affected: if modified { 1 } else { 0 },
        })
    }

    /// Bulk update multiple tools
    pub async fn bulk_update_tools(&self, update: BulkUpdateRequest) -> Result<ToolManagementResult> {
        let mut affected_tools = Vec::new();
        let mut file_changes: HashMap<PathBuf, CapabilityFile> = HashMap::new();

        // Group tools by file for efficient processing
        for tool_name in &update.tool_names {
            match self.find_tool_file(tool_name).await {
                Ok(file_path) => {
                    // Load file if not already loaded
                    if !file_changes.contains_key(&file_path) {
                        let capability_file = self.load_capability_file(&file_path).await?;
                        file_changes.insert(file_path.clone(), capability_file);
                    }

                    // Modify the tool in the loaded file
                    if let Some(capability_file) = file_changes.get_mut(&file_path) {
                        let mut modified = false;

                        if let Some(enabled) = update.enabled {
                            if capability_file.set_tool_enabled(tool_name, enabled).is_ok() {
                                modified = true;
                            }
                        }

                        if let Some(hidden) = update.hidden {
                            if capability_file.set_tool_hidden(tool_name, hidden).is_ok() {
                                modified = true;
                            }
                        }

                        if modified {
                            affected_tools.push(tool_name.clone());
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to find tool '{}': {}", tool_name, e);
                }
            }
        }

        // Save all modified files
        for (file_path, capability_file) in file_changes {
            self.save_capability_file(&file_path, &capability_file)?;
        }

        let actions = match (update.enabled, update.hidden) {
            (Some(true), Some(false)) => "enabled and shown".to_string(),
            (Some(true), Some(true)) => "enabled and hidden".to_string(),
            (Some(false), Some(false)) => "disabled and shown".to_string(),
            (Some(false), Some(true)) => "disabled and hidden".to_string(),
            (Some(true), None) => "enabled".to_string(),
            (Some(false), None) => "disabled".to_string(),
            (None, Some(true)) => "hidden".to_string(),
            (None, Some(false)) => "shown".to_string(),
            (None, None) => "unchanged".to_string(),
        };

        info!("Bulk updated {} tools: {}", affected_tools.len(), actions);

        // Trigger registry reload for immediate effect if any tools were affected
        if !affected_tools.is_empty() {
            self.trigger_registry_reload().await?;
        }

        let affected_count = affected_tools.len();
        Ok(ToolManagementResult {
            success: true,
            message: format!("Successfully {} {} tools", actions, affected_count),
            affected_tools,
            total_affected: affected_count,
        })
    }

    /// Hide all tools globally
    pub async fn hide_all_tools(&self) -> Result<ToolManagementResult> {
        let capability_files = self.discover_capability_files().await?;
        let mut total_changed = 0;

        for file_path in capability_files {
            let mut capability_file = self.load_capability_file(&file_path).await?;
            let visible_count = capability_file.visible_tool_count();
            if visible_count > 0 {
                capability_file.set_all_tools_hidden(true);
                self.save_capability_file(&file_path, &capability_file)?;
                total_changed += visible_count;
                info!("Hidden {} tools in: {}", visible_count, file_path.display());
            }
        }

        // Trigger registry reload for immediate effect if any tools were changed
        if total_changed > 0 {
            self.trigger_registry_reload().await?;
        }

        Ok(ToolManagementResult {
            success: true,
            message: format!("Successfully hidden {} tools across all capability files", total_changed),
            affected_tools: vec![], // Too many to list individually
            total_affected: total_changed,
        })
    }

    /// Show all tools globally
    pub async fn show_all_tools(&self) -> Result<ToolManagementResult> {
        let capability_files = self.discover_capability_files().await?;
        let mut total_changed = 0;

        for file_path in capability_files {
            let mut capability_file = self.load_capability_file(&file_path).await?;
            let hidden_count = capability_file.hidden_tool_count();
            if hidden_count > 0 {
                capability_file.set_all_tools_hidden(false);
                self.save_capability_file(&file_path, &capability_file)?;
                total_changed += hidden_count;
                info!("Shown {} tools in: {}", hidden_count, file_path.display());
            }
        }

        // Trigger registry reload for immediate effect if any tools were changed
        if total_changed > 0 {
            self.trigger_registry_reload().await?;
        }

        Ok(ToolManagementResult {
            success: true,
            message: format!("Successfully shown {} tools across all capability files", total_changed),
            affected_tools: vec![], // Too many to list individually
            total_affected: total_changed,
        })
    }

    /// Enable all tools globally
    pub async fn enable_all_tools(&self) -> Result<ToolManagementResult> {
        let capability_files = self.discover_capability_files().await?;
        let mut total_changed = 0;

        for file_path in capability_files {
            let mut capability_file = self.load_capability_file(&file_path).await?;
            let disabled_count = capability_file.disabled_tool_count();
            if disabled_count > 0 {
                capability_file.set_all_tools_enabled(true);
                self.save_capability_file(&file_path, &capability_file)?;
                total_changed += disabled_count;
                info!("Enabled {} tools in: {}", disabled_count, file_path.display());
            }
        }

        // Trigger registry reload for immediate effect if any tools were changed
        if total_changed > 0 {
            self.trigger_registry_reload().await?;
        }

        Ok(ToolManagementResult {
            success: true,
            message: format!("Successfully enabled {} tools across all capability files", total_changed),
            affected_tools: vec![], // Too many to list individually
            total_affected: total_changed,
        })
    }

    /// Disable all tools globally
    pub async fn disable_all_tools(&self) -> Result<ToolManagementResult> {
        let capability_files = self.discover_capability_files().await?;
        let mut total_changed = 0;

        for file_path in capability_files {
            let mut capability_file = self.load_capability_file(&file_path).await?;
            let enabled_count = capability_file.enabled_tool_count();
            if enabled_count > 0 {
                capability_file.set_all_tools_enabled(false);
                self.save_capability_file(&file_path, &capability_file)?;
                total_changed += enabled_count;
                info!("Disabled {} tools in: {}", enabled_count, file_path.display());
            }
        }

        // Trigger registry reload for immediate effect if any tools were changed
        if total_changed > 0 {
            self.trigger_registry_reload().await?;
        }

        Ok(ToolManagementResult {
            success: true,
            message: format!("Successfully disabled {} tools across all capability files", total_changed),
            affected_tools: vec![], // Too many to list individually
            total_affected: total_changed,
        })
    }

    /// Get statistics about tool states
    pub async fn get_tool_statistics(&self) -> Result<serde_json::Value> {
        let capability_files = self.discover_capability_files().await?;
        let mut total_tools = 0;
        let mut visible_tools = 0;
        let mut hidden_tools = 0;
        let mut enabled_tools = 0;
        let mut disabled_tools = 0;
        let mut active_tools = 0;

        for file_path in &capability_files {
            let capability_file = self.load_capability_file(file_path).await?;
            total_tools += capability_file.tool_count();
            visible_tools += capability_file.visible_tool_count();
            hidden_tools += capability_file.hidden_tool_count();
            enabled_tools += capability_file.enabled_tool_count();
            disabled_tools += capability_file.disabled_tool_count();
            active_tools += capability_file.active_tools().len();
        }

        Ok(serde_json::json!({
            "total_tools": total_tools,
            "visible_tools": visible_tools,
            "hidden_tools": hidden_tools,
            "enabled_tools": enabled_tools,
            "disabled_tools": disabled_tools,
            "active_tools": active_tools,
            "total_files": capability_files.len(),
            "visibility_percentage": if total_tools > 0 { (visible_tools as f64 / total_tools as f64) * 100.0 } else { 0.0 },
            "enabled_percentage": if total_tools > 0 { (enabled_tools as f64 / total_tools as f64) * 100.0 } else { 0.0 }
        }))
    }

    // Private helper methods (reusing CLI logic)

    async fn refresh_capability_cache(&self) -> Result<()> {
        let capability_files = self.discover_capability_files().await?;
        let mut cache = self.capability_cache.write().await;
        cache.clear();

        for file_path in capability_files {
            let capability_file = self.load_capability_file(&file_path).await?;
            for tool in &capability_file.tools {
                cache.insert(tool.name.clone(), file_path.clone());
            }
        }

        Ok(())
    }

    async fn find_tool_file(&self, tool_name: &str) -> Result<PathBuf> {
        // Try cache first
        {
            let cache = self.capability_cache.read().await;
            if let Some(file_path) = cache.get(tool_name) {
                return Ok(file_path.clone());
            }
        }

        // Cache miss - refresh and try again
        self.refresh_capability_cache().await?;
        
        let cache = self.capability_cache.read().await;
        cache.get(tool_name).cloned()
            .ok_or_else(|| ProxyError::registry(format!("Tool '{}' not found in any capability file", tool_name)))
    }

    async fn discover_capability_files(&self) -> Result<Vec<PathBuf>> {
        let capabilities_paths = &self.config.registry.paths;
        let mut capability_files = Vec::new();

        fn scan_directory(dir: &std::path::Path, files: &mut Vec<PathBuf>) -> Result<()> {
            if dir.is_dir() {
                for entry in fs::read_dir(dir).map_err(|e| {
                    ProxyError::registry(format!("Failed to read capabilities directory {}: {}", dir.display(), e))
                })? {
                    let entry = entry.map_err(|e| {
                        ProxyError::registry(format!("Failed to read directory entry: {}", e))
                    })?;
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
                        files.push(path);
                    } else if path.is_dir() {
                        scan_directory(&path, files)?;
                    }
                }
            }
            Ok(())
        }

        for path_str in capabilities_paths {
            let path = PathBuf::from(path_str);
            if path.is_file() && path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
                capability_files.push(path);
            } else if path.is_dir() {
                scan_directory(&path, &mut capability_files)?;
            }
        }
        Ok(capability_files)
    }

    async fn load_capability_file(&self, path: &PathBuf) -> Result<CapabilityFile> {
        // Use the registry's loader to handle both enhanced and legacy formats
        self.registry_loader.load_file(path).await
    }


    fn save_capability_file(&self, file_path: &PathBuf, capability_file: &CapabilityFile) -> Result<()> {
        let yaml_content = serde_yaml::to_string(capability_file).map_err(|e| {
            ProxyError::registry(format!("Failed to serialize capability file: {}", e))
        })?;
        
        fs::write(file_path, yaml_content).map_err(|e| {
            ProxyError::registry(format!("Failed to write file {}: {}", file_path.display(), e))
        })?;
        
        debug!("Saved capability file: {}", file_path.display());
        Ok(())
    }
}