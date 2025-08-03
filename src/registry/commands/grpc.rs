//! gRPC Generator Adapter
//! 
//! This module provides an adapter for the gRPC capability generator
//! that implements the CapabilityGeneratorBase trait.

use crate::error::{ProxyError, Result};
use crate::registry::generator_common::CapabilityGeneratorBase;
use crate::registry::grpc_generator::{GrpcCapabilityGenerator, GrpcGeneratorConfig, StreamingStrategy};
use crate::registry::types::CapabilityFile;
use std::path::Path;
use std::collections::HashMap;

/// gRPC Generator Adapter
/// 
/// Adapts the GrpcCapabilityGenerator to implement the CapabilityGeneratorBase trait
/// for use with the unified CLI.
pub struct GrpcGeneratorAdapter {
    /// The underlying gRPC generator
    generator: GrpcCapabilityGenerator,
}

impl GrpcGeneratorAdapter {
    /// Create a new gRPC generator adapter
    pub fn new(endpoint: String) -> Self {
        let config = GrpcGeneratorConfig {
            endpoint,
            auth_config: None,
            tool_prefix: None,
            service_filter: None,
            method_filter: None,
            server_streaming_strategy: StreamingStrategy::Polling,
            client_streaming_strategy: StreamingStrategy::Polling,
            bidirectional_streaming_strategy: StreamingStrategy::Polling,
            include_method_options: false,
            separate_streaming_tools: false,
            use_enhanced_format: true, // Always use enhanced format
        };
        
        Self {
            generator: GrpcCapabilityGenerator::new(config),
        }
    }

    /// Set authentication configuration
    pub fn with_auth(mut self, common_auth_config: crate::registry::generator_common::AuthConfig) -> Self {
        // Convert from generator_common::AuthType to grpc_generator::AuthType
        let grpc_auth_type = match common_auth_config.auth_type {
            crate::registry::generator_common::AuthType::None => {
                crate::registry::grpc_generator::AuthType::None
            },
            crate::registry::generator_common::AuthType::ApiKey { key, header } => {
                crate::registry::grpc_generator::AuthType::ApiKey { key, header }
            },
            crate::registry::generator_common::AuthType::Bearer { token } => {
                crate::registry::grpc_generator::AuthType::Bearer { token: token.clone() }
            },
            crate::registry::generator_common::AuthType::Basic { username, password } => {
                crate::registry::grpc_generator::AuthType::Basic { username, password }
            },
            crate::registry::generator_common::AuthType::OAuth { token, token_type } => {
                crate::registry::grpc_generator::AuthType::OAuth { token, token_type }
            },
        };
        
        // Create gRPC-specific AuthConfig
        let grpc_auth_config = crate::registry::grpc_generator::AuthConfig {
            auth_type: grpc_auth_type,
            headers: common_auth_config.headers,
        };
        
        let mut config = self.generator.config.clone();
        config.auth_config = Some(grpc_auth_config);
        self.generator = GrpcCapabilityGenerator::new(config);
        self
    }

    /// Set tool name prefix
    pub fn with_prefix(mut self, prefix: String) -> Self {
        let mut config = self.generator.config.clone();
        config.tool_prefix = Some(prefix);
        self.generator = GrpcCapabilityGenerator::new(config);
        self
    }

    /// Set service filter
    pub fn with_service_filter(mut self, filter: Vec<String>) -> Self {
        let mut config = self.generator.config.clone();
        config.service_filter = Some(filter);
        self.generator = GrpcCapabilityGenerator::new(config);
        self
    }

    /// Set method filter
    pub fn with_method_filter(mut self, filter: Vec<String>) -> Self {
        let mut config = self.generator.config.clone();
        config.method_filter = Some(filter);
        self.generator = GrpcCapabilityGenerator::new(config);
        self
    }

    /// Set server streaming strategy
    pub fn with_server_streaming_strategy(mut self, strategy: StreamingStrategy) -> Self {
        let mut config = self.generator.config.clone();
        config.server_streaming_strategy = strategy;
        self.generator = GrpcCapabilityGenerator::new(config);
        self
    }

    /// Set client streaming strategy
    pub fn with_client_streaming_strategy(mut self, strategy: StreamingStrategy) -> Self {
        let mut config = self.generator.config.clone();
        config.client_streaming_strategy = strategy;
        self.generator = GrpcCapabilityGenerator::new(config);
        self
    }

    /// Set bidirectional streaming strategy
    pub fn with_bidirectional_streaming_strategy(mut self, strategy: StreamingStrategy) -> Self {
        let mut config = self.generator.config.clone();
        config.bidirectional_streaming_strategy = strategy;
        self.generator = GrpcCapabilityGenerator::new(config);
        self
    }

    /// Set whether to include method options
    pub fn with_include_method_options(mut self, include: bool) -> Self {
        let mut config = self.generator.config.clone();
        config.include_method_options = include;
        self.generator = GrpcCapabilityGenerator::new(config);
        self
    }

    /// Set whether to generate separate streaming tools
    pub fn with_separate_streaming_tools(mut self, separate: bool) -> Self {
        let mut config = self.generator.config.clone();
        config.separate_streaming_tools = separate;
        self.generator = GrpcCapabilityGenerator::new(config);
        self
    }
}

impl CapabilityGeneratorBase for GrpcGeneratorAdapter {
    fn generate_from_content(&self, content: &str) -> Result<CapabilityFile> {
        self.generator.generate_from_proto_content(content)
    }
    
    fn name(&self) -> &str {
        "grpc"
    }
    
    fn description(&self) -> &str {
        "gRPC Capability Generator"
    }
    
    fn supported_extensions(&self) -> Vec<&str> {
        vec!["proto"]
    }
}