//! Tests for the generator adapters used by the unified CLI
//!
//! These tests validate the adapter classes for each generator type (GraphQL, gRPC, OpenAPI)
//! that implement the CapabilityGeneratorBase trait for use with the unified CLI.

use magictunnel::registry::commands::{
    GraphQLGeneratorAdapter, GrpcGeneratorAdapter, OpenAPIGeneratorAdapter
};
use magictunnel::registry::generator_common::{
    CapabilityGeneratorBase, AuthConfig, AuthType
};
use magictunnel::registry::graphql_generator::AuthConfig as GraphQLAuthConfig;
use magictunnel::registry::openapi_generator::NamingConvention;
use magictunnel::error::Result;
use std::collections::HashMap;

#[test]
fn test_graphql_adapter_creation() {
    let endpoint = "https://graphql.example.com";
    let adapter = GraphQLGeneratorAdapter::new(endpoint.to_string());
    
    assert_eq!(adapter.name(), "graphql");
    assert_eq!(adapter.description(), "GraphQL Capability Generator");
    assert!(adapter.supported_extensions().contains(&"graphql"));
    assert!(adapter.supported_extensions().contains(&"gql"));
    assert!(adapter.supported_extensions().contains(&"json"));
}

#[test]
fn test_graphql_adapter_with_prefix() {
    let endpoint = "https://graphql.example.com";
    let prefix = "test";
    let adapter = GraphQLGeneratorAdapter::new(endpoint.to_string())
        .with_prefix(prefix.to_string());
    
    // We can't directly test the prefix is applied since it's internal,
    // but we can verify the adapter is still valid
    assert_eq!(adapter.name(), "graphql");
}

#[test]
fn test_graphql_adapter_with_auth() {
    let endpoint = "https://graphql.example.com";
    
    // Test with Bearer auth
    let auth_config = GraphQLAuthConfig {
        auth_type: magictunnel::registry::graphql_generator::AuthType::Bearer { token: "test-token".to_string() },
        headers: HashMap::new(),
    };
    
    let adapter = GraphQLGeneratorAdapter::new(endpoint.to_string())
        .with_auth(auth_config);
    
    assert_eq!(adapter.name(), "graphql");
}

#[test]
fn test_grpc_adapter_creation() {
    let endpoint = "grpc.example.com:50051";
    let adapter = GrpcGeneratorAdapter::new(endpoint.to_string());
    
    assert_eq!(adapter.name(), "grpc");
    assert_eq!(adapter.description(), "gRPC Capability Generator");
    assert!(adapter.supported_extensions().contains(&"proto"));
}

#[test]
fn test_grpc_adapter_with_prefix() {
    let endpoint = "grpc.example.com:50051";
    let prefix = "test";
    let adapter = GrpcGeneratorAdapter::new(endpoint.to_string())
        .with_prefix(prefix.to_string());
    
    assert_eq!(adapter.name(), "grpc");
}

#[test]
fn test_grpc_adapter_with_service_filter() {
    let endpoint = "grpc.example.com:50051";
    let services = vec!["UserService".to_string(), "ProductService".to_string()];
    let adapter = GrpcGeneratorAdapter::new(endpoint.to_string())
        .with_service_filter(services);
    
    assert_eq!(adapter.name(), "grpc");
}

#[test]
fn test_grpc_adapter_with_method_filter() {
    let endpoint = "grpc.example.com:50051";
    let methods = vec!["GetUser".to_string(), "ListProducts".to_string()];
    let adapter = GrpcGeneratorAdapter::new(endpoint.to_string())
        .with_method_filter(methods);
    
    assert_eq!(adapter.name(), "grpc");
}

#[test]
fn test_grpc_adapter_with_streaming_strategies() {
    let endpoint = "grpc.example.com:50051";
    let adapter = GrpcGeneratorAdapter::new(endpoint.to_string())
        .with_server_streaming_strategy(magictunnel::registry::grpc_generator::StreamingStrategy::Polling)
        .with_client_streaming_strategy(magictunnel::registry::grpc_generator::StreamingStrategy::Pagination)
        .with_bidirectional_streaming_strategy(magictunnel::registry::grpc_generator::StreamingStrategy::AgentLevel);
    
    assert_eq!(adapter.name(), "grpc");
}

#[test]
fn test_grpc_adapter_with_options() {
    let endpoint = "grpc.example.com:50051";
    let adapter = GrpcGeneratorAdapter::new(endpoint.to_string())
        .with_include_method_options(true)
        .with_separate_streaming_tools(true);
    
    assert_eq!(adapter.name(), "grpc");
}

#[test]
fn test_openapi_adapter_creation() {
    let base_url = "https://api.example.com";
    let adapter = OpenAPIGeneratorAdapter::new(base_url.to_string());
    
    assert_eq!(adapter.name(), "openapi");
    assert_eq!(adapter.description(), "OpenAPI Capability Generator");
    assert!(adapter.supported_extensions().contains(&"json"));
    assert!(adapter.supported_extensions().contains(&"yaml"));
    assert!(adapter.supported_extensions().contains(&"yml"));
}

#[test]
fn test_openapi_adapter_with_prefix() {
    let base_url = "https://api.example.com";
    let prefix = "test";
    let adapter = OpenAPIGeneratorAdapter::new(base_url.to_string())
        .with_prefix(prefix.to_string());
    
    assert_eq!(adapter.name(), "openapi");
}

#[test]
fn test_openapi_adapter_with_naming_convention() {
    let base_url = "https://api.example.com";
    
    // Test with OperationId naming convention
    let adapter = OpenAPIGeneratorAdapter::new(base_url.to_string())
        .with_naming_convention(NamingConvention::OperationId);
    
    assert_eq!(adapter.name(), "openapi");
    
    // Test with MethodPath naming convention
    let adapter = OpenAPIGeneratorAdapter::new(base_url.to_string())
        .with_naming_convention(NamingConvention::MethodPath);
    
    assert_eq!(adapter.name(), "openapi");
}

#[test]
fn test_openapi_adapter_with_method_filter() {
    let base_url = "https://api.example.com";
    let methods = vec!["GET".to_string(), "POST".to_string()];
    let adapter = OpenAPIGeneratorAdapter::new(base_url.to_string())
        .with_method_filter(methods);
    
    assert_eq!(adapter.name(), "openapi");
}

#[test]
fn test_openapi_adapter_with_include_deprecated() {
    let base_url = "https://api.example.com";
    let adapter = OpenAPIGeneratorAdapter::new(base_url.to_string())
        .with_include_deprecated(true);
    
    assert_eq!(adapter.name(), "openapi");
}

#[test]
fn test_openapi_adapter_with_auth() {
    let base_url = "https://api.example.com";
    
    // Test with Bearer auth
    let auth_config = magictunnel::registry::openapi_generator::AuthConfig {
        auth_type: magictunnel::registry::openapi_generator::AuthType::Bearer { token: "test-token".to_string() },
        headers: HashMap::new(),
    };
    
    let adapter = OpenAPIGeneratorAdapter::new(base_url.to_string())
        .with_auth(auth_config);
    
    assert_eq!(adapter.name(), "openapi");
}