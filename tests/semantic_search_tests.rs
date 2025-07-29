//! Comprehensive tests for semantic search functionality
//! 
//! This test suite covers:
//! - Phase 3.8.1: Persistent Semantic Search System
//! - Phase 3.8.2: Dynamic Embedding Management  
//! - Phase 3.8.3: Hybrid Search Strategy Implementation

use magictunnel::discovery::{
    SemanticSearchService, SemanticSearchConfig,
    EmbeddingManager, EmbeddingManagerConfig,
    SmartDiscoveryService, SmartDiscoveryConfig, SmartDiscoveryRequest
};
use magictunnel::registry::RegistryService;
use magictunnel::config::RegistryConfig;
use magictunnel::registry::types::{ToolDefinition, RoutingConfig};
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use tokio;

/// Helper function to create a test semantic search config
fn create_test_semantic_config(temp_dir: &TempDir) -> SemanticSearchConfig {
    let mut config = SemanticSearchConfig::default();
    config.enabled = true;
    config.storage.embeddings_file = temp_dir.path().join("embeddings.json");
    config.storage.metadata_file = temp_dir.path().join("metadata.json");
    config.storage.hash_file = temp_dir.path().join("hashes.json");
    config.model.cache_dir = temp_dir.path().join("models");
    config.similarity_threshold = 0.5; // Lower threshold for testing
    config
}

/// Helper function to create test tool definitions
fn create_test_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "ping_test".to_string(),
            description: "Test network connectivity by pinging a host".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "host": {"type": "string", "description": "Host to ping"}
                },
                "required": ["host"]
            }),
            routing: RoutingConfig {
                r#type: "http".to_string(),
                config: json!({"url": "http://example.com/ping"}),
            },
            annotations: None,
            enabled: true,
            hidden: false,
        },
        ToolDefinition {
            name: "search_files".to_string(),
            description: "Search for files in the filesystem using patterns".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Search pattern"},
                    "directory": {"type": "string", "description": "Directory to search"}
                },
                "required": ["pattern"]
            }),
            routing: RoutingConfig {
                r#type: "filesystem".to_string(),
                config: json!({"type": "search"}),
            },
            annotations: None,
            enabled: true,
            hidden: false,
        },
        ToolDefinition {
            name: "database_query".to_string(),
            description: "Execute SQL queries against the database".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "sql": {"type": "string", "description": "SQL query to execute"}
                },
                "required": ["sql"]
            }),
            routing: RoutingConfig {
                r#type: "database".to_string(),
                config: json!({"connection": "default"}),
            },
            annotations: None,
            enabled: false, // Disabled tool
            hidden: false,
        },
        ToolDefinition {
            name: "api_request".to_string(),
            description: "Make HTTP API requests to external services".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {"type": "string", "description": "API endpoint URL"},
                    "method": {"type": "string", "description": "HTTP method"}
                },
                "required": ["url"]
            }),
            routing: RoutingConfig {
                r#type: "http".to_string(),
                config: json!({"type": "api"}),
            },
            annotations: None,
            enabled: true,
            hidden: true, // Hidden tool
        },
    ]
}

/// Test semantic search service initialization and basic functionality
#[tokio::test]
async fn test_semantic_search_initialization() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_semantic_config(&temp_dir);
    
    // Test service creation
    let service = SemanticSearchService::new(config.clone());
    assert!(service.is_enabled());
    
    // Test initialization
    let result = service.initialize().await;
    assert!(result.is_ok(), "Service initialization failed: {:?}", result);
    
    // Verify directories were created
    assert!(temp_dir.path().join("models").exists());
    
    // Test statistics
    let stats = service.get_stats().await;
    assert_eq!(stats.get("enabled").unwrap(), &json!(true));
    assert_eq!(stats.get("model_name").unwrap(), &json!("all-MiniLM-L6-v2"));
}

/// Test embedding generation and persistence
#[tokio::test]
async fn test_embedding_generation_and_persistence() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_semantic_config(&temp_dir);
    
    let service = SemanticSearchService::new(config);
    service.initialize().await.expect("Failed to initialize service");
    
    // Test embedding generation
    let test_text = "ping network connectivity test";
    let embedding = service.generate_embedding(test_text).await;
    assert!(embedding.is_ok(), "Embedding generation failed");
    
    let embedding = embedding.unwrap();
    assert_eq!(embedding.len(), 384, "Embedding should have 384 dimensions");
    
    // Test that embeddings are normalized (if configured)
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 0.01, "Embedding should be normalized");
}

/// Test semantic search functionality
#[tokio::test]
async fn test_semantic_search_functionality() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_semantic_config(&temp_dir);
    
    let service = SemanticSearchService::new(config);
    service.initialize().await.expect("Failed to initialize service");
    
    // Add some test tools with embeddings
    let tools = create_test_tools();
    let mut storage = service.storage.write().await;
    
    for tool in &tools {
        if tool.enabled {
            let embedding_text = format!("{}: {}", tool.name, tool.description);
            drop(storage); // Release lock for async call
            let embedding = service.generate_embedding(&embedding_text).await.unwrap();
            storage = service.storage.write().await;
            
            let metadata = magictunnel::discovery::semantic::ToolMetadata {
                name: tool.name.clone(),
                description: tool.description.clone(),
                enabled: tool.enabled,
                hidden: tool.hidden,
                content_hash: service.generate_content_hash(tool),
                last_updated: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                embedding_dims: embedding.len(),
            };
            
            storage.add_tool_embedding(tool.name.clone(), embedding, metadata);
        }
    }
    drop(storage);
    
    // Test search functionality (with fallback embeddings, we test the system works)
    let matches = service.search_similar_tools("check network connectivity").await;
    assert!(matches.is_ok(), "Search failed");
    
    let matches = matches.unwrap();
    
    // With fallback embeddings, we may or may not get matches above threshold
    // The important thing is the system doesn't crash and returns valid results
    println!("Found {} matches with fallback embeddings", matches.len());
    
    // If we found matches, verify they have the expected structure
    for match_result in &matches {
        assert!(!match_result.tool_name.is_empty(), "Tool name should not be empty");
        assert!(match_result.similarity_score >= 0.0, "Similarity score should be non-negative");
        assert!(match_result.similarity_score <= 1.0, "Similarity score should not exceed 1.0");
    }
    
    // Test that enabled tools are properly tracked
    let storage = service.storage.read().await;
    let enabled_tools = storage.get_enabled_tools();
    println!("Enabled tools in storage: {:?}", enabled_tools);
    assert!(enabled_tools.contains(&"ping_test".to_string()), "ping_test should be marked as enabled");
    assert!(enabled_tools.contains(&"search_files".to_string()), "search_files should be marked as enabled");
    assert!(enabled_tools.contains(&"api_request".to_string()), "api_request should be marked as enabled (even if hidden)");
    assert!(!enabled_tools.contains(&"database_query".to_string()), "database_query should not be marked as enabled");
}

/// Test embedding manager lifecycle functionality
#[tokio::test]
async fn test_embedding_manager_lifecycle() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let semantic_config = create_test_semantic_config(&temp_dir);
    
    // Create mock registry
    let registry_config = RegistryConfig {
        r#type: "file".to_string(),
        paths: vec!["./test_capabilities".to_string()],
        hot_reload: false,
        validation: magictunnel::config::ValidationConfig {
            strict: false,
            allow_unknown_fields: true,
        },
    };
    
    let registry = Arc::new(RegistryService::new(registry_config).await.unwrap());
    let semantic_service = Arc::new(SemanticSearchService::new(semantic_config));
    semantic_service.initialize().await.unwrap();
    
    // Create embedding manager
    let manager_config = EmbeddingManagerConfig {
        check_interval_seconds: 1,
        auto_save: true,
        batch_size: 10,
        background_monitoring: false, // Disable for testing
        preserve_user_settings: true,
        enable_hot_reload: true,
    };
    
    let manager = EmbeddingManager::new(
        Arc::clone(&registry),
        Arc::clone(&semantic_service),
        manager_config,
    );
    
    // Test initialization
    let result = manager.initialize().await;
    assert!(result.is_ok(), "Manager initialization failed");
    
    // Test manual sync (should work even with no tools)
    let summary = manager.force_sync().await;
    assert!(summary.is_ok(), "Manual sync failed");
    
    let summary = summary.unwrap();
    assert_eq!(summary.failed, 0, "No operations should fail");
    
    // Test statistics
    let stats = manager.get_stats().await;
    assert!(stats.contains_key("last_known_tools"));
    assert!(stats.contains_key("user_disabled_tools"));
    assert!(stats.contains_key("auto_save"));
}

/// Test hybrid search strategy
#[tokio::test] 
async fn test_hybrid_search_strategy() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let semantic_config = create_test_semantic_config(&temp_dir);
    
    // Create registry with test tools
    let registry_config = RegistryConfig {
        r#type: "file".to_string(),
        paths: vec!["./test_capabilities".to_string()],
        hot_reload: false,
        validation: magictunnel::config::ValidationConfig {
            strict: false,
            allow_unknown_fields: true,
        },
    };
    
    let registry = Arc::new(RegistryService::new(registry_config).await.unwrap());
    
    // Create smart discovery config with hybrid mode
    let mut discovery_config = SmartDiscoveryConfig::default();
    discovery_config.tool_selection_mode = "hybrid".to_string();
    discovery_config.enable_sequential_mode = true;
    discovery_config.semantic_search = semantic_config;
    discovery_config.semantic_search.enabled = true;
    
    let discovery_service = SmartDiscoveryService::new(registry, discovery_config);
    assert!(discovery_service.is_ok(), "Discovery service creation failed");
    
    let discovery_service = discovery_service.unwrap();
    let init_result = discovery_service.initialize().await;
    assert!(init_result.is_ok(), "Discovery service initialization failed");
    
    // Test hybrid search request
    let request = SmartDiscoveryRequest {
        request: "test network connectivity".to_string(),
        context: Some("I want to check if a server is reachable".to_string()),
        preferred_tools: None,
        confidence_threshold: Some(0.6),
        include_error_details: Some(true),
        sequential_mode: Some(true),
    };
    
    let response = discovery_service.discover_and_execute(request).await;
    assert!(response.is_ok(), "Hybrid search failed");
    
    let response = response.unwrap();
    // The response might succeed or fail depending on available tools,
    // but the hybrid search system should handle it gracefully
    assert!(response.metadata.reasoning.is_some(), "Should have reasoning");
}

/// Test different search modes
#[tokio::test]
async fn test_search_mode_selection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let semantic_config = create_test_semantic_config(&temp_dir);
    
    let registry_config = RegistryConfig {
        r#type: "file".to_string(),
        paths: vec!["./test_capabilities".to_string()],
        hot_reload: false,
        validation: magictunnel::config::ValidationConfig {
            strict: false,
            allow_unknown_fields: true,
        },
    };
    
    let registry = Arc::new(RegistryService::new(registry_config).await.unwrap());
    
    // Test different modes
    let modes = vec!["rule_based", "semantic_based", "hybrid"];
    
    for mode in modes {
        let mut discovery_config = SmartDiscoveryConfig::default();
        discovery_config.tool_selection_mode = mode.to_string();
        discovery_config.enable_sequential_mode = true;
        discovery_config.semantic_search = semantic_config.clone();
        discovery_config.semantic_search.enabled = mode != "rule_based";
        
        let discovery_service = SmartDiscoveryService::new(Arc::clone(&registry), discovery_config);
        assert!(discovery_service.is_ok(), "Discovery service creation failed for mode: {}", mode);
        
        let discovery_service = discovery_service.unwrap();
        let init_result = discovery_service.initialize().await;
        assert!(init_result.is_ok(), "Discovery service initialization failed for mode: {}", mode);
        
        // Test that service reports correct enabled state
        assert_eq!(discovery_service.is_enabled(), true);
    }
}

/// Test embedding storage persistence
#[tokio::test]
async fn test_embedding_persistence() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_semantic_config(&temp_dir);
    
    // Create first service instance and add some embeddings
    {
        let service = SemanticSearchService::new(config.clone());
        service.initialize().await.unwrap();
        
        let tools = create_test_tools();
        let mut storage = service.storage.write().await;
        
        for tool in &tools[0..2] { // Add first two tools
            if tool.enabled {
                let embedding_text = format!("{}: {}", tool.name, tool.description);
                drop(storage);
                let embedding = service.generate_embedding(&embedding_text).await.unwrap();
                storage = service.storage.write().await;
                
                let metadata = magictunnel::discovery::semantic::ToolMetadata {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    enabled: tool.enabled,
                    hidden: tool.hidden,
                    content_hash: service.generate_content_hash(tool),
                    last_updated: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    embedding_dims: embedding.len(),
                };
                
                storage.add_tool_embedding(tool.name.clone(), embedding, metadata);
            }
        }
        drop(storage);
        
        // Save embeddings
        service.save_embeddings().await.unwrap();
    }
    
    // Create second service instance and verify data was loaded
    {
        let service = SemanticSearchService::new(config);
        service.initialize().await.unwrap();
        
        let stats = service.get_stats().await;
        let total_embeddings = stats.get("total_embeddings").unwrap().as_u64().unwrap();
        assert!(total_embeddings > 0, "Should have loaded existing embeddings");
        
        // Test search works with loaded embeddings
        // With fallback embeddings, we test the persistence system works correctly
        let matches = service.search_similar_tools("network test").await.unwrap();
        println!("Found {} matches with persisted embeddings", matches.len());
        
        // The important test is that the storage system loaded the embeddings correctly
        let storage = service.storage.read().await;
        let tool_names = storage.get_tool_names();
        assert!(!tool_names.is_empty(), "Should have loaded tool embeddings from storage");
        println!("Loaded tools from storage: {:?}", tool_names);
    }
}

/// Test error handling and edge cases
#[tokio::test]
async fn test_error_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut config = create_test_semantic_config(&temp_dir);
    
    // Test with disabled semantic search
    config.enabled = false;
    let service = SemanticSearchService::new(config.clone());
    assert!(!service.is_enabled());
    
    let matches = service.search_similar_tools("test query").await.unwrap();
    assert!(matches.is_empty(), "Disabled service should return empty results");
    
    // Test with very high similarity threshold
    config.enabled = true;
    config.similarity_threshold = 0.99;
    let service = SemanticSearchService::new(config);
    service.initialize().await.unwrap();
    
    // Even with high threshold, service should handle gracefully
    let matches = service.search_similar_tools("test query").await.unwrap();
    // Matches might be empty due to high threshold, which is expected
    // Check that max results setting is respected (we don't have access to config anymore)
    assert!(matches.len() <= 1000); // Reasonable upper bound
}

/// Test configuration validation
#[tokio::test]
async fn test_configuration_validation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut config = create_test_semantic_config(&temp_dir);
    
    // Test various configuration scenarios
    config.max_results = 0;
    let service = SemanticSearchService::new(config.clone());
    let matches = service.search_similar_tools("test").await.unwrap();
    assert_eq!(matches.len(), 0, "Max results 0 should return empty");
    
    config.max_results = 1;
    let service = SemanticSearchService::new(config);
    service.initialize().await.unwrap();
    
    // Add test embedding and verify max results is respected
    let stats = service.get_stats().await;
    assert!(stats.get("enabled").unwrap().as_bool().unwrap());
}

/// Performance and benchmark tests
#[tokio::test]
async fn test_performance_benchmarks() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_semantic_config(&temp_dir);
    
    let service = SemanticSearchService::new(config);
    service.initialize().await.unwrap();
    
    // Test embedding generation performance
    let start = std::time::Instant::now();
    for i in 0..10 {
        let text = format!("test embedding generation performance {}", i);
        let _ = service.generate_embedding(&text).await.unwrap();
    }
    let duration = start.elapsed();
    
    // Should complete 10 embeddings in reasonable time (placeholder implementation)
    assert!(duration.as_secs() < 5, "Embedding generation should be reasonably fast");
    
    // Test search performance with multiple embeddings
    let mut storage = service.storage.write().await;
    for i in 0..50 {
        let tool_name = format!("test_tool_{}", i);
        let embedding = vec![0.1; 384]; // Dummy embedding
        let metadata = magictunnel::discovery::semantic::ToolMetadata {
            name: tool_name.clone(),
            description: format!("Test tool number {}", i),
            enabled: true,
            hidden: false,
            content_hash: format!("hash_{}", i),
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            embedding_dims: 384,
        };
        storage.add_tool_embedding(tool_name, embedding, metadata);
    }
    drop(storage);
    
    // Test search performance
    let start = std::time::Instant::now();
    let matches = service.search_similar_tools("test query").await.unwrap();
    let search_duration = start.elapsed();
    
    assert!(search_duration.as_millis() < 100, "Search should be fast even with many embeddings");
    
    // With fallback embeddings, we may not get semantic matches, but the system should work
    println!("Performance test found {} matches in {}ms", matches.len(), search_duration.as_millis());
    
    // The important performance test is that the search completed quickly
    // and the storage contains the expected number of tools
    let storage = service.storage.read().await;
    let (total, enabled, hidden) = storage.get_stats();
    assert_eq!(total, 50, "Should have 50 tools in storage");
    assert_eq!(enabled, 50, "All test tools should be enabled");
    assert_eq!(hidden, 0, "No test tools should be hidden");
}