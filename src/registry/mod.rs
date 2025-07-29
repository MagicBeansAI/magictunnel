//! Capability registry for managing tool definitions and routing


pub mod commands;
pub mod generator_common;
pub mod generator_config;
pub mod graphql_generator;
pub mod grpc_generator;
pub mod loader;
pub mod openapi_generator;
pub mod service;
pub mod tool_aggregation;
pub mod types;


pub use commands::{
    GraphQLGeneratorAdapter, GrpcGeneratorAdapter, OpenAPIGeneratorAdapter,
    CapabilityMerger, CapabilityValidator
};
pub use generator_common::{AuthConfig, AuthType, CapabilityGenerator, GeneratorRegistry};
pub use generator_config::GeneratorConfigFile;
pub use loader::RegistryLoader;
pub use service::{RegistryService, CapabilityRegistry, RegistryMetadata};
pub use tool_aggregation::{ToolAggregationService, AggregatedTool, AggregationStats};
pub use types::*;
