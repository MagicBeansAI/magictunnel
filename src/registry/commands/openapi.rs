//! OpenAPI Generator Adapter
//! 
//! This module provides an adapter for the OpenAPI capability generator
//! that implements the CapabilityGeneratorBase trait.

use crate::error::{ProxyError, Result};
use crate::registry::generator_common::CapabilityGeneratorBase;
use crate::registry::openapi_generator::{OpenAPICapabilityGenerator, AuthConfig, NamingConvention};
use crate::registry::types::CapabilityFile;
use std::path::Path;

/// OpenAPI Generator Adapter
/// 
/// Adapts the OpenAPICapabilityGenerator to implement the CapabilityGeneratorBase trait
/// for use with the unified CLI.
pub struct OpenAPIGeneratorAdapter {
    /// The underlying OpenAPI generator
    generator: OpenAPICapabilityGenerator,
}

impl OpenAPIGeneratorAdapter {
    /// Create a new OpenAPI generator adapter
    pub fn new(base_url: String) -> Self {
        Self {
            generator: OpenAPICapabilityGenerator::new(base_url),
        }
    }

    /// Set authentication configuration
    pub fn with_auth(mut self, auth_config: AuthConfig) -> Self {
        self.generator = self.generator.with_auth(auth_config);
        self
    }

    /// Set tool name prefix
    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.generator = self.generator.with_prefix(prefix);
        self
    }

    /// Set operation filter (by operation ID or tag)
    pub fn with_operation_filter(mut self, filter: Vec<String>) -> Self {
        self.generator = self.generator.with_operation_filter(filter);
        self
    }

    /// Set path filter (regex patterns)
    pub fn with_path_filter(mut self, filter: Vec<String>) -> Self {
        self.generator = self.generator.with_path_filter(filter);
        self
    }

    /// Set HTTP method filter
    pub fn with_method_filter(mut self, filter: Vec<String>) -> Self {
        self.generator = self.generator.with_method_filter(filter);
        self
    }

    /// Set naming convention
    pub fn with_naming_convention(mut self, convention: NamingConvention) -> Self {
        self.generator = self.generator.with_naming_convention(convention);
        self
    }

    /// Include deprecated operations
    pub fn with_include_deprecated(mut self, include_deprecated: bool) -> Self {
        if include_deprecated {
            self.generator = self.generator.include_deprecated();
        }
        self
    }
}

impl CapabilityGeneratorBase for OpenAPIGeneratorAdapter {
    fn generate_from_content(&self, content: &str) -> Result<CapabilityFile> {
        // Clone the generator to avoid borrowing issues with mutable methods
        let mut generator = OpenAPICapabilityGenerator::new(self.generator.base_url.clone());
        
        // Copy configuration from the original generator
        if let Some(auth_config) = &self.generator.auth_config {
            generator = generator.with_auth(auth_config.clone());
        }
        
        if let Some(prefix) = &self.generator.tool_prefix {
            generator = generator.with_prefix(prefix.clone());
        }
        
        if let Some(operation_filter) = &self.generator.operation_filter {
            generator = generator.with_operation_filter(operation_filter.clone());
        }
        
        if let Some(path_filter) = &self.generator.path_filter {
            generator = generator.with_path_filter(path_filter.clone());
        }
        
        if let Some(method_filter) = &self.generator.method_filter {
            generator = generator.with_method_filter(method_filter.clone());
        }
        
        generator = generator.with_naming_convention(self.generator.naming_convention.clone());
        
        if self.generator.include_deprecated {
            generator = generator.include_deprecated();
        }
        
        // Generate from the content
        generator.generate_from_spec(content)
    }
    
    fn name(&self) -> &str {
        "openapi"
    }
    
    fn description(&self) -> &str {
        "OpenAPI Capability Generator"
    }
    
    fn supported_extensions(&self) -> Vec<&str> {
        vec!["json", "yaml", "yml", "openapi", "swagger"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_openapi_adapter_creation() {
        let adapter = OpenAPIGeneratorAdapter::new("https://api.example.com".to_string());
        assert_eq!(adapter.name(), "openapi");
        assert_eq!(adapter.description(), "OpenAPI Capability Generator");
        assert_eq!(adapter.supported_extensions(), vec!["json", "yaml", "yml", "openapi", "swagger"]);
    }
    
    #[test]
    fn test_openapi_adapter_with_prefix() {
        let adapter = OpenAPIGeneratorAdapter::new("https://api.example.com".to_string())
            .with_prefix("test".to_string());
        
        assert_eq!(adapter.generator.tool_prefix, Some("test".to_string()));
    }
}