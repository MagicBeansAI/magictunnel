//! Comprehensive tests for the gRPC capability generator implementation
//! 
//! These tests validate the core functionality of the gRPC capability generator,
//! including protobuf parsing, service definition extraction, message type parsing,
//! streaming semantics, and authentication handling.

use magictunnel::registry::grpc_generator::{
    GrpcCapabilityGenerator, GrpcGeneratorConfig, StreamingStrategy,
    AuthConfig, AuthType
};
use magictunnel::registry::types::CapabilityFile;
use std::collections::HashMap;
use std::path::Path;
use std::fs;

/// Test basic generator creation
#[test]
fn test_generator_creation() {
    // Create a generator with default configuration
    let config = GrpcGeneratorConfig {
        endpoint: "localhost:50051".to_string(),
        auth_config: None,
        tool_prefix: None,
        service_filter: None,
        method_filter: None,
        server_streaming_strategy: StreamingStrategy::Polling,
        client_streaming_strategy: StreamingStrategy::Polling,
        bidirectional_streaming_strategy: StreamingStrategy::Polling,
        include_method_options: false,
        separate_streaming_tools: false,
        use_enhanced_format: false,
    };
    
    let generator = GrpcCapabilityGenerator::new(config);
    assert_eq!(generator.config.endpoint, "localhost:50051");
}

/// Test proto file generation
#[test]
fn test_proto_file_generation() {
    // Create a generator with default configuration
    let config = GrpcGeneratorConfig {
        endpoint: "localhost:50051".to_string(),
        auth_config: None,
        tool_prefix: Some("test".to_string()),
        service_filter: None,
        method_filter: None,
        server_streaming_strategy: StreamingStrategy::Polling,
        client_streaming_strategy: StreamingStrategy::Polling,
        bidirectional_streaming_strategy: StreamingStrategy::Polling,
        include_method_options: false,
        separate_streaming_tools: false,
        use_enhanced_format: false,
    };
    
    let generator = GrpcCapabilityGenerator::new(config);
    
    // Path to the test proto file
    let proto_path = Path::new("data/grpc_test/comprehensive_test_service.proto");
    
    // This should panic with "not yet implemented" until we implement it
    let _capability_file = generator.generate_from_proto_file(proto_path).unwrap();
}

/// Test proto content generation
#[test]
fn test_proto_content_generation() {
    // Create a generator with default configuration
    let config = GrpcGeneratorConfig {
        endpoint: "localhost:50051".to_string(),
        auth_config: None,
        tool_prefix: None,
        service_filter: None,
        method_filter: None,
        server_streaming_strategy: StreamingStrategy::Polling,
        client_streaming_strategy: StreamingStrategy::Polling,
        bidirectional_streaming_strategy: StreamingStrategy::Polling,
        include_method_options: false,
        separate_streaming_tools: false,
        use_enhanced_format: false,
    };
    
    let generator = GrpcCapabilityGenerator::new(config);
    
    // Simple proto content for testing
    let proto_content = r#"
        syntax = "proto3";
        
        package test.simple;
        
        service SimpleService {
            rpc GetItem(GetItemRequest) returns (GetItemResponse) {}
            rpc ListItems(ListItemsRequest) returns (stream ListItemsResponse) {}
        }
        
        message GetItemRequest {
            string item_id = 1;
        }
        
        message GetItemResponse {
            string item_id = 1;
            string name = 2;
            int32 quantity = 3;
        }
        
        message ListItemsRequest {
            int32 page_size = 1;
            string page_token = 2;
        }
        
        message ListItemsResponse {
            repeated Item items = 1;
            string next_page_token = 2;
        }
        
        message Item {
            string id = 1;
            string name = 2;
            int32 quantity = 3;
        }
    "#;
    
    // This should panic with "not yet implemented" until we implement it
    let _capability_file = generator.generate_from_proto_content(proto_content).unwrap();
}

/// Test tool name generation
#[test]
fn test_tool_name_generation() {
    // Create a generator with a prefix
    let config = GrpcGeneratorConfig {
        endpoint: "localhost:50051".to_string(),
        auth_config: None,
        tool_prefix: Some("test".to_string()),
        service_filter: None,
        method_filter: None,
        server_streaming_strategy: StreamingStrategy::Polling,
        client_streaming_strategy: StreamingStrategy::Polling,
        bidirectional_streaming_strategy: StreamingStrategy::Polling,
        include_method_options: false,
        separate_streaming_tools: false,
        use_enhanced_format: false,
    };
    
    let generator = GrpcCapabilityGenerator::new(config);
    
    // Test the generate_tool_name method
    let service_name = "UserService";
    let method_name = "GetUser";
    let expected_name = "test_userservice_getuser";
    
    // We can test this because generate_tool_name is public
    let tool_name = generator.generate_tool_name(
        &magictunnel::registry::grpc_generator::GrpcService {
            name: service_name.to_string(),
            package: "test.user".to_string(),
            methods: Vec::new(),
            options: HashMap::new(),
        },
        &magictunnel::registry::grpc_generator::GrpcMethod {
            name: method_name.to_string(),
            input_type: "GetUserRequest".to_string(),
            output_type: "GetUserResponse".to_string(),
            client_streaming: false,
            server_streaming: false,
            options: HashMap::new(),
        }
    );
    
    assert_eq!(tool_name, expected_name, "Tool name should be correctly generated with prefix");
    
    // Test without prefix
    let config = GrpcGeneratorConfig {
        endpoint: "localhost:50051".to_string(),
        auth_config: None,
        tool_prefix: None,
        service_filter: None,
        method_filter: None,
        server_streaming_strategy: StreamingStrategy::Polling,
        client_streaming_strategy: StreamingStrategy::Polling,
        bidirectional_streaming_strategy: StreamingStrategy::Polling,
        include_method_options: false,
        separate_streaming_tools: false,
        use_enhanced_format: false,
    };
    
    let generator = GrpcCapabilityGenerator::new(config);
    
    let expected_name = "userservice_getuser";
    let tool_name = generator.generate_tool_name(
        &magictunnel::registry::grpc_generator::GrpcService {
            name: service_name.to_string(),
            package: "test.user".to_string(),
            methods: Vec::new(),
            options: HashMap::new(),
        },
        &magictunnel::registry::grpc_generator::GrpcMethod {
            name: method_name.to_string(),
            input_type: "GetUserRequest".to_string(),
            output_type: "GetUserResponse".to_string(),
            client_streaming: false,
            server_streaming: false,
            options: HashMap::new(),
        }
    );
    
    assert_eq!(tool_name, expected_name, "Tool name should be correctly generated without prefix");
}

/// Test input schema generation
#[test]
fn test_input_schema_generation() {
    // Create a generator with default configuration
    let config = GrpcGeneratorConfig {
        endpoint: "localhost:50051".to_string(),
        auth_config: None,
        tool_prefix: None,
        service_filter: None,
        method_filter: None,
        server_streaming_strategy: StreamingStrategy::Polling,
        client_streaming_strategy: StreamingStrategy::Polling,
        bidirectional_streaming_strategy: StreamingStrategy::Polling,
        include_method_options: false,
        separate_streaming_tools: false,
        use_enhanced_format: false,
    };
    
    let generator = GrpcCapabilityGenerator::new(config);
    
    // Create a mock service and method
    let service = magictunnel::registry::grpc_generator::GrpcService {
        name: "UserService".to_string(),
        package: "test.user".to_string(),
        methods: Vec::new(),
        options: HashMap::new(),
    };
    
    let method = magictunnel::registry::grpc_generator::GrpcMethod {
        name: "GetUser".to_string(),
        input_type: "GetUserRequest".to_string(),
        output_type: "GetUserResponse".to_string(),
        client_streaming: false,
        server_streaming: false,
        options: HashMap::new(),
    };
    
    // This should panic with "not yet implemented" until we implement it
    let _tool_definition = generator.method_to_tool_definition(&service, &method).unwrap();
}

/// Test authentication configuration
#[test]
fn test_authentication_configuration() {
    // Test different authentication types
    let auth_configs = vec![
        (
            "API Key",
            AuthConfig {
                auth_type: AuthType::ApiKey {
                    key: "test-api-key".to_string(),
                    header: "X-API-Key".to_string(),
                },
                headers: HashMap::new(),
            }
        ),
        (
            "Bearer",
            AuthConfig {
                auth_type: AuthType::Bearer { token: "test-bearer-token".to_string() },
                headers: HashMap::new(),
            }
        ),
        (
            "Basic",
            AuthConfig {
                auth_type: AuthType::Basic {
                    username: "testuser".to_string(),
                    password: "testpass".to_string(),
                },
                headers: HashMap::new(),
            }
        ),
        (
            "OAuth",
            AuthConfig {
                auth_type: AuthType::OAuth {
                    token: "test-oauth-token".to_string(),
                    token_type: "Bearer".to_string(),
                },
                headers: HashMap::new(),
            }
        ),
    ];
    
    for (auth_name, auth_config) in auth_configs {
        // Create a generator with the current auth config
        let config = GrpcGeneratorConfig {
            endpoint: "localhost:50051".to_string(),
            auth_config: Some(auth_config.clone()),
            tool_prefix: None,
            service_filter: None,
            method_filter: None,
            server_streaming_strategy: StreamingStrategy::Polling,
            client_streaming_strategy: StreamingStrategy::Polling,
            bidirectional_streaming_strategy: StreamingStrategy::Polling,
            include_method_options: false,
            separate_streaming_tools: false,
            use_enhanced_format: false,
        };
        
        let generator = GrpcCapabilityGenerator::new(config);
        
        // Test routing config creation
        let service = magictunnel::registry::grpc_generator::GrpcService {
            name: "AuthService".to_string(),
            package: "test.auth".to_string(),
            methods: Vec::new(),
            options: HashMap::new(),
        };
        
        let method = magictunnel::registry::grpc_generator::GrpcMethod {
            name: "GetSecureData".to_string(),
            input_type: "SecureDataRequest".to_string(),
            output_type: "SecureDataResponse".to_string(),
            client_streaming: false,
            server_streaming: false,
            options: HashMap::new(),
        };
        
        let routing_config = generator.create_routing_config(&service, &method).unwrap();
        
        // Validate authentication in routing config
        if let serde_json::Value::Object(config_map) = &routing_config.config {
            assert!(config_map.contains_key("headers"), "Routing config should have headers");
            
            if let Some(serde_json::Value::Object(headers)) = config_map.get("headers") {
                match auth_config.auth_type {
                    AuthType::ApiKey { ref header, .. } => {
                        assert!(headers.contains_key(header), "Headers should contain API key header");
                    },
                    AuthType::Bearer { .. } | AuthType::OAuth { .. } => {
                        assert!(headers.contains_key("Authorization"), "Headers should contain Authorization header");
                    },
                    AuthType::Basic { .. } => {
                        assert!(headers.contains_key("Authorization"), "Headers should contain Authorization header");
                        if let Some(auth_header) = headers.get("Authorization").and_then(|v| v.as_str()) {
                            assert!(auth_header.starts_with("Basic "), "Authorization header should start with 'Basic '");
                        } else {
                            panic!("Authorization header should be a string");
                        }
                    },
                    AuthType::None => {
                        // No authentication headers expected
                    },
                }
            } else {
                panic!("Headers should be an object");
            }
        } else {
            panic!("Routing config should be an object");
        }
    }
}

/// Test streaming strategy configuration
#[test]
fn test_streaming_strategy_configuration() {
    // Test with different streaming strategies
    let test_strategies = vec![
        (StreamingStrategy::Polling, "polling"),
        (StreamingStrategy::Pagination, "pagination"),
        (StreamingStrategy::AgentLevel, "agent-level"),
    ];
    
    for (strategy, strategy_name) in test_strategies {
        // Create a generator with the current strategy
        let config = GrpcGeneratorConfig {
            endpoint: "localhost:50051".to_string(),
            auth_config: None,
            tool_prefix: None,
            service_filter: None,
            method_filter: None,
            server_streaming_strategy: strategy.clone(),
            client_streaming_strategy: strategy.clone(),
            bidirectional_streaming_strategy: strategy.clone(),
            include_method_options: false,
            separate_streaming_tools: false,
            use_enhanced_format: false,
        };
        
        let generator = GrpcCapabilityGenerator::new(config);
        
        // Verify the strategy is set correctly
        assert_eq!(
            format!("{:?}", generator.config.server_streaming_strategy),
            format!("{:?}", strategy),
            "Server streaming strategy should be set correctly"
        );
        
        assert_eq!(
            format!("{:?}", generator.config.client_streaming_strategy),
            format!("{:?}", strategy),
            "Client streaming strategy should be set correctly"
        );
        
        assert_eq!(
            format!("{:?}", generator.config.bidirectional_streaming_strategy),
            format!("{:?}", strategy),
            "Bidirectional streaming strategy should be set correctly"
        );
    }
}

/// Test comprehensive proto file parsing
#[test]
fn test_comprehensive_proto_parsing() {
    // Create a generator with default configuration
    let config = GrpcGeneratorConfig {
        endpoint: "localhost:50051".to_string(),
        auth_config: None,
        tool_prefix: Some("test".to_string()),
        service_filter: None,
        method_filter: None,
        server_streaming_strategy: StreamingStrategy::Polling,
        client_streaming_strategy: StreamingStrategy::Polling,
        bidirectional_streaming_strategy: StreamingStrategy::Polling,
        include_method_options: true,
        separate_streaming_tools: false,
        use_enhanced_format: false,
    };
    
    let generator = GrpcCapabilityGenerator::new(config);
    
    // Path to the comprehensive test proto file
    let proto_path = Path::new("data/grpc_test/comprehensive_test_service.proto");
    
    // This should panic with "not yet implemented" until we implement it
    let _capability_file = generator.generate_from_proto_file(proto_path).unwrap();
}

/// Test streaming service proto file parsing
#[test]
fn test_streaming_proto_parsing() {
    // Create a generator with default configuration
    let config = GrpcGeneratorConfig {
        endpoint: "localhost:50051".to_string(),
        auth_config: None,
        tool_prefix: Some("stream".to_string()),
        service_filter: None,
        method_filter: None,
        server_streaming_strategy: StreamingStrategy::Polling,
        client_streaming_strategy: StreamingStrategy::Polling,
        bidirectional_streaming_strategy: StreamingStrategy::Polling,
        include_method_options: true,
        separate_streaming_tools: true,
        use_enhanced_format: false,
    };
    
    let generator = GrpcCapabilityGenerator::new(config);
    
    // Path to the streaming test proto file
    let proto_path = Path::new("data/grpc_test/comprehensive_test_streaming.proto");
    
    // This should panic with "not yet implemented" until we implement it
    let _capability_file = generator.generate_from_proto_file(proto_path).unwrap();
}

/// Test authenticated service proto file parsing
#[test]
fn test_auth_proto_parsing() {
    // Create a generator with API Key authentication
    let mut headers = HashMap::new();
    headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());
    
    let config = GrpcGeneratorConfig {
        endpoint: "localhost:50051".to_string(),
        auth_config: Some(AuthConfig {
            auth_type: AuthType::ApiKey {
                key: "test-api-key".to_string(),
                header: "X-API-Key".to_string(),
            },
            headers,
        }),
        tool_prefix: Some("auth".to_string()),
        service_filter: None,
        method_filter: None,
        server_streaming_strategy: StreamingStrategy::Polling,
        client_streaming_strategy: StreamingStrategy::Polling,
        bidirectional_streaming_strategy: StreamingStrategy::Polling,
        include_method_options: true,
        separate_streaming_tools: false,
        use_enhanced_format: false,
    };
    
    let generator = GrpcCapabilityGenerator::new(config);
    
    // Path to the auth test proto file
    let proto_path = Path::new("data/grpc_test/comprehensive_test_auth.proto");
    
    // This should panic with "not yet implemented" until we implement it
    let _capability_file = generator.generate_from_proto_file(proto_path).unwrap();
}

/// Test service filtering
#[test]
fn test_service_filtering() {
    // Create a generator with service filter
    let config = GrpcGeneratorConfig {
        endpoint: "localhost:50051".to_string(),
        auth_config: None,
        tool_prefix: None,
        service_filter: Some(vec!["UserService".to_string()]),
        method_filter: None,
        server_streaming_strategy: StreamingStrategy::Polling,
        client_streaming_strategy: StreamingStrategy::Polling,
        bidirectional_streaming_strategy: StreamingStrategy::Polling,
        include_method_options: false,
        separate_streaming_tools: false,
        use_enhanced_format: false,
    };
    
    let generator = GrpcCapabilityGenerator::new(config);
    
    // Verify the service filter is set correctly
    assert_eq!(
        generator.config.service_filter.as_ref().unwrap()[0],
        "UserService",
        "Service filter should be set correctly"
    );
}

/// Test method filtering
#[test]
fn test_method_filtering() {
    // Create a generator with method filter
    let config = GrpcGeneratorConfig {
        endpoint: "localhost:50051".to_string(),
        auth_config: None,
        tool_prefix: None,
        service_filter: None,
        method_filter: Some(vec!["GetUser".to_string(), "ListUsers".to_string()]),
        server_streaming_strategy: StreamingStrategy::Polling,
        client_streaming_strategy: StreamingStrategy::Polling,
        bidirectional_streaming_strategy: StreamingStrategy::Polling,
        include_method_options: false,
        separate_streaming_tools: false,
        use_enhanced_format: false,
    };
    
    let generator = GrpcCapabilityGenerator::new(config);
    
    // Verify the method filter is set correctly
    assert_eq!(
        generator.config.method_filter.as_ref().unwrap()[0],
        "GetUser",
        "Method filter should be set correctly"
    );
    
    assert_eq!(
        generator.config.method_filter.as_ref().unwrap()[1],
        "ListUsers",
        "Method filter should be set correctly"
    );
}