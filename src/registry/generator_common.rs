//! Common functionality for capability generators
//! 
//! This module provides shared traits, structures, and utilities for all capability generators.

use crate::error::{ProxyError, Result};
use crate::registry::types::{CapabilityFile, ToolDefinition, FileMetadata};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Common authentication configuration for all generators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication type
    pub auth_type: AuthType,
    /// Additional headers to include in requests
    pub headers: HashMap<String, String>,
}

/// Supported authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthType {
    /// No authentication
    #[serde(rename = "none")]
    None,
    /// API Key authentication
    #[serde(rename = "api_key")]
    ApiKey {
        /// API key value
        key: String,
        /// Header name for the API key
        header: String,
    },
    /// Bearer token authentication
    #[serde(rename = "bearer")]
    Bearer {
        /// Bearer token value
        token: String,
    },
    /// Basic authentication
    #[serde(rename = "basic")]
    Basic {
        /// Username
        username: String,
        /// Password
        password: String,
    },
    /// OAuth 2.0 authentication
    #[serde(rename = "oauth")]
    OAuth {
        /// OAuth token
        token: String,
        /// Token type (usually "Bearer")
        token_type: String,
    },
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            auth_type: AuthType::None,
            headers: HashMap::new(),
        }
    }
}

/// Base trait for capability generators with object-safe methods
pub trait CapabilityGeneratorBase {
    /// Generate a capability file from source content
    fn generate_from_content(&self, content: &str) -> Result<CapabilityFile>;
    
    /// Get the generator name
    fn name(&self) -> &str;
    
    /// Get the generator description
    fn description(&self) -> &str;
    
    /// Get supported file extensions
    fn supported_extensions(&self) -> Vec<&str>;
}

/// Trait for capability generators with additional utility methods
pub trait CapabilityGenerator: CapabilityGeneratorBase {
    /// Generate a capability file from a source file
    fn generate_from_file<P: AsRef<Path>>(&self, file_path: P) -> Result<CapabilityFile> {
        let path = file_path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| ProxyError::config(format!("Failed to read file '{}': {}", path.display(), e)))?;
        
        self.generate_from_content(&content)
    }
    
    /// Check if a file is supported
    fn supports_file<P: AsRef<Path>>(&self, file_path: P) -> bool {
        let path = file_path.as_ref();
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                return self.supported_extensions().contains(&ext_str);
            }
        }
        false
    }
    
    /// Create a standard file metadata
    fn create_metadata(&self, name: &str, description: &str) -> FileMetadata {
        FileMetadata::with_name(name.to_string())
            .description(description.to_string())
            .version("1.0.0".to_string())
            .author("MCP Generator".to_string())
            .tags(vec![self.name().to_string()])
    }
}

// Implement CapabilityGenerator for any type that implements CapabilityGeneratorBase
impl<T: CapabilityGeneratorBase> CapabilityGenerator for T {}

/// Base configuration for all generators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseGeneratorConfig {
    /// Tool name prefix
    pub tool_prefix: Option<String>,
    /// Authentication configuration
    pub auth_config: Option<AuthConfig>,
    /// Output file path
    pub output_path: Option<String>,
}

impl Default for BaseGeneratorConfig {
    fn default() -> Self {
        Self {
            tool_prefix: None,
            auth_config: None,
            output_path: None,
        }
    }
}

/// Helper function to read file content
pub fn read_file_content<P: AsRef<Path>>(file_path: P) -> Result<String> {
    std::fs::read_to_string(&file_path)
        .map_err(|e| ProxyError::config(format!(
            "Failed to read file '{}': {}", 
            file_path.as_ref().display(), 
            e
        )))
}

/// Helper function to write capability file to YAML
pub fn write_capability_file<P: AsRef<Path>>(capability_file: &CapabilityFile, output_path: P) -> Result<()> {
    let yaml_content = serde_yaml::to_string(capability_file)
        .map_err(|e| ProxyError::config(format!("Failed to serialize to YAML: {}", e)))?;

    std::fs::write(&output_path, yaml_content)
        .map_err(|e| ProxyError::config(format!(
            "Failed to write output file '{}': {}",
            output_path.as_ref().display(),
            e
        )))?;

    Ok(())
}

/// Helper function to detect file format from extension
pub fn detect_file_format<P: AsRef<Path>>(file_path: P) -> Option<&'static str> {
    let path = file_path.as_ref();
    if let Some(extension) = path.extension() {
        match extension.to_str() {
            Some("json") => Some("json"),
            Some("yaml") | Some("yml") => Some("yaml"),
            Some("proto") => Some("proto"),
            Some("graphql") | Some("gql") => Some("graphql"),
            _ => None,
        }
    } else {
        None
    }
}

/// Helper function to detect content format from content
pub fn detect_content_format(content: &str) -> &'static str {
    let trimmed = content.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        "json"
    } else if trimmed.starts_with("type ") || trimmed.starts_with("schema ") || trimmed.starts_with("query ") {
        "graphql"
    } else if trimmed.starts_with("syntax ") || trimmed.starts_with("package ") || trimmed.starts_with("service ") {
        "proto"
    } else {
        "yaml" // Default to YAML
    }
}

/// Registry of available generators
pub struct GeneratorRegistry {
    generators: HashMap<String, Box<dyn CapabilityGeneratorBase>>,
}

impl GeneratorRegistry {
    /// Create a new generator registry
    pub fn new() -> Self {
        Self {
            generators: HashMap::new(),
        }
    }
    
    /// Register a generator
    pub fn register<G: CapabilityGeneratorBase + 'static>(&mut self, generator: G) {
        self.generators.insert(generator.name().to_string(), Box::new(generator));
    }
    
    /// Get a generator by name
    pub fn get(&self, name: &str) -> Option<&dyn CapabilityGeneratorBase> {
        self.generators.get(name).map(|g| g.as_ref())
    }
    
    /// Get all registered generators
    pub fn all(&self) -> Vec<&dyn CapabilityGeneratorBase> {
        self.generators.values().map(|g| g.as_ref()).collect()
    }
    
    /// Find a generator that supports a file
    pub fn find_for_file<P: AsRef<Path>>(&self, file_path: P) -> Option<&dyn CapabilityGeneratorBase> {
        let path = file_path.as_ref();
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                return self.all().into_iter().find(|g| g.supported_extensions().contains(&ext_str));
            }
        }
        None
    }
}

impl Default for GeneratorRegistry {
    fn default() -> Self {
        Self::new()
    }
}