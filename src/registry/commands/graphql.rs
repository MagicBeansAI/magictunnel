//! GraphQL Generator Adapter
//! 
//! This module provides an adapter for the GraphQL capability generator
//! that implements the CapabilityGeneratorBase trait.

use crate::error::{ProxyError, Result};
use crate::registry::generator_common::CapabilityGeneratorBase;
use crate::registry::graphql_generator::{GraphQLCapabilityGenerator, AuthConfig, AuthType};
use crate::registry::types::CapabilityFile;
use std::path::Path;
use std::collections::HashMap;

/// GraphQL Generator Adapter
/// 
/// Adapts the GraphQLCapabilityGenerator to implement the CapabilityGeneratorBase trait
/// for use with the unified CLI.
pub struct GraphQLGeneratorAdapter {
    /// The underlying GraphQL generator
    generator: GraphQLCapabilityGenerator,
    /// Endpoint URL for GraphQL API
    endpoint_url: String,
    /// Authentication configuration
    auth_config: Option<AuthConfig>,
    /// Tool name prefix
    prefix: Option<String>,
}

impl GraphQLGeneratorAdapter {
    /// Create a new GraphQL generator adapter
    pub fn new(endpoint: String) -> Self {
        Self {
            generator: GraphQLCapabilityGenerator::new(endpoint.clone()),
            endpoint_url: endpoint,
            auth_config: None,
            prefix: None,
        }
    }

    /// Set authentication configuration
    pub fn with_auth(mut self, auth_config: AuthConfig) -> Self {
        self.auth_config = Some(auth_config.clone());
        self.generator = self.generator.with_auth(auth_config);
        self
    }

    /// Set tool name prefix
    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix.clone());
        self.generator = self.generator.with_prefix(prefix);
        self
    }

    /// Set whether to include deprecated fields and operations
    pub fn with_include_deprecated(mut self, include_deprecated: bool) -> Self {
        // This would need to be implemented in the GraphQLCapabilityGenerator
        // For now, we'll just log a warning
        eprintln!("Warning: include_deprecated not implemented for GraphQLCapabilityGenerator");
        self
    }

    /// Set whether to include descriptions in schemas
    pub fn with_include_descriptions(mut self, include_descriptions: bool) -> Self {
        // This would need to be implemented in the GraphQLCapabilityGenerator
        // For now, we'll just log a warning
        eprintln!("Warning: include_descriptions not implemented for GraphQLCapabilityGenerator");
        self
    }

    /// Set whether to generate separate tools for mutations and queries
    pub fn with_separate_mutation_query(mut self, separate: bool) -> Self {
        // This would need to be implemented in the GraphQLCapabilityGenerator
        // For now, we'll just log a warning
        eprintln!("Warning: separate_mutation_query not implemented for GraphQLCapabilityGenerator");
        self
    }
}

impl CapabilityGeneratorBase for GraphQLGeneratorAdapter {
    fn generate_from_content(&self, content: &str) -> Result<CapabilityFile> {
        // Detect format (SDL or JSON introspection)
        if content.trim_start().starts_with('{') {
            // Create a new generator to avoid borrowing issues
            let mut generator = GraphQLCapabilityGenerator::new(self.endpoint_url.clone());
            
            // Copy configuration from the original generator
            if let Some(prefix) = &self.prefix {
                generator = generator.with_prefix(prefix.clone());
            }
            
            if let Some(auth_config) = &self.auth_config {
                generator = generator.with_auth(auth_config.clone());
            }
            
            generator.generate_from_introspection(content)
        } else {
            // Create a new generator to avoid borrowing issues
            let mut generator = GraphQLCapabilityGenerator::new(self.endpoint_url.clone());
            
            // Copy configuration from the original generator
            if let Some(prefix) = &self.prefix {
                generator = generator.with_prefix(prefix.clone());
            }
            
            if let Some(auth_config) = &self.auth_config {
                generator = generator.with_auth(auth_config.clone());
            }
            
            generator.generate_from_sdl(content)
        }
    }
    
    fn name(&self) -> &str {
        "graphql"
    }
    
    fn description(&self) -> &str {
        "GraphQL Capability Generator"
    }
    
    fn supported_extensions(&self) -> Vec<&str> {
        vec!["graphql", "gql", "json"]
    }
}