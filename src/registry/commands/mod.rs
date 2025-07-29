//! Command adapters for capability generators
//! 
//! This module provides adapter classes for each generator type that implement
//! the CapabilityGenerator trait, allowing them to be used with the unified CLI.

pub mod graphql;
pub mod grpc;
pub mod openapi;
pub mod merge;
pub mod validate;

pub use graphql::GraphQLGeneratorAdapter;
pub use grpc::GrpcGeneratorAdapter;
pub use openapi::OpenAPIGeneratorAdapter;
pub use self::merge::{CapabilityMerger, MergeStrategy};
pub use self::validate::CapabilityValidator;