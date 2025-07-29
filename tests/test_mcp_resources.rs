//! Integration tests for MCP resource management

use magictunnel::error::Result;
use magictunnel::mcp::resources::{ResourceManager, FileResourceProvider, ResourceProvider};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::fs;
use base64::{Engine as _, engine::general_purpose};

/// Test basic resource manager functionality
#[tokio::test]
async fn test_resource_manager_basic() -> Result<()> {
    let manager = ResourceManager::new();
    
    // Initially no providers
    assert_eq!(manager.provider_count().await, 0);
    
    // List resources should return empty
    let (resources, cursor) = manager.list_resources(None).await?;
    assert!(resources.is_empty());
    assert!(cursor.is_none());
    
    Ok(())
}

/// Test file resource provider with temporary directory
#[tokio::test]
async fn test_file_resource_provider() -> Result<()> {
    // Create temporary directory with test files
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    
    // Create test files
    fs::write(temp_path.join("test.txt"), "Hello, World!").await.unwrap();
    fs::write(temp_path.join("data.json"), r#"{"key": "value"}"#).await.unwrap();
    fs::write(temp_path.join("binary.png"), vec![0x89, 0x50, 0x4E, 0x47]).await.unwrap(); // PNG header
    
    // Create file provider
    let provider = Arc::new(FileResourceProvider::new(temp_path, "file:".to_string())?);

    // Test listing resources
    let (resources, cursor) = provider.list_resources(None).await?;
    assert_eq!(resources.len(), 3);
    assert!(cursor.is_none());

    // Verify resource properties
    let txt_resource = resources.iter().find(|r| r.name == "test.txt").unwrap();
    assert_eq!(txt_resource.uri, "file:/test.txt");
    assert_eq!(txt_resource.mime_type, Some("text/plain".to_string()));

    let json_resource = resources.iter().find(|r| r.name == "data.json").unwrap();
    assert_eq!(json_resource.uri, "file:/data.json");
    assert_eq!(json_resource.mime_type, Some("application/json".to_string()));

    let png_resource = resources.iter().find(|r| r.name == "binary.png").unwrap();
    assert_eq!(png_resource.uri, "file:/binary.png");
    assert_eq!(png_resource.mime_type, Some("image/png".to_string()));
    
    Ok(())
}

/// Test reading text and binary resources
#[tokio::test]
async fn test_resource_reading() -> Result<()> {
    // Create temporary directory with test files
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    
    let text_content = "Hello, Resource World!";
    let binary_content = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG header
    
    fs::write(temp_path.join("test.txt"), text_content).await.unwrap();
    fs::write(temp_path.join("test.png"), &binary_content).await.unwrap();
    
    // Create file provider
    let provider = Arc::new(FileResourceProvider::new(temp_path, "file:".to_string())?);

    // Test reading text resource
    let text_resource = provider.read_resource("file:/test.txt").await?;
    assert_eq!(text_resource.uri, "file:/test.txt");
    assert_eq!(text_resource.mime_type, Some("text/plain".to_string()));
    assert_eq!(text_resource.text, Some(text_content.to_string()));
    assert!(text_resource.blob.is_none());
    assert!(text_resource.is_text());

    // Test reading binary resource
    let binary_resource = provider.read_resource("file:/test.png").await?;
    assert_eq!(binary_resource.uri, "file:/test.png");
    assert_eq!(binary_resource.mime_type, Some("image/png".to_string()));
    assert!(binary_resource.text.is_none());
    assert!(binary_resource.blob.is_some());
    assert!(binary_resource.is_blob());
    
    // Verify binary content
    let decoded = general_purpose::STANDARD.decode(binary_resource.blob.unwrap()).unwrap();
    assert_eq!(decoded, binary_content);
    
    Ok(())
}

/// Test resource manager with multiple providers
#[tokio::test]
async fn test_resource_manager_multiple_providers() -> Result<()> {
    // Create two temporary directories
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    
    // Create test files in each directory
    fs::write(temp_dir1.path().join("file1.txt"), "Content 1").await.unwrap();
    fs::write(temp_dir2.path().join("file2.txt"), "Content 2").await.unwrap();
    
    // Create providers
    let provider1 = Arc::new(FileResourceProvider::new(temp_dir1.path(), "dir1:".to_string())?);
    let provider2 = Arc::new(FileResourceProvider::new(temp_dir2.path(), "dir2:".to_string())?);

    // Create resource manager and add providers
    let manager = ResourceManager::new();
    manager.add_provider(provider1).await;
    manager.add_provider(provider2).await;
    
    assert_eq!(manager.provider_count().await, 2);
    
    // List all resources
    let (resources, _) = manager.list_resources(None).await?;
    assert_eq!(resources.len(), 2);
    
    // Verify resources from both providers
    let resource1 = resources.iter().find(|r| r.uri == "dir1:/file1.txt").unwrap();
    let resource2 = resources.iter().find(|r| r.uri == "dir2:/file2.txt").unwrap();

    assert_eq!(resource1.name, "file1.txt");
    assert_eq!(resource2.name, "file2.txt");

    // Test reading from specific providers
    let content1 = manager.read_resource("dir1:/file1.txt").await?;
    let content2 = manager.read_resource("dir2:/file2.txt").await?;
    
    assert_eq!(content1.text, Some("Content 1".to_string()));
    assert_eq!(content2.text, Some("Content 2".to_string()));
    
    Ok(())
}

/// Test error handling for invalid resources
#[tokio::test]
async fn test_resource_error_handling() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let provider = Arc::new(FileResourceProvider::new(temp_dir.path(), "file:".to_string())?);

    let manager = ResourceManager::new();
    manager.add_provider(provider).await;
    
    // Test reading non-existent resource
    let result = manager.read_resource("file:/nonexistent.txt").await;
    assert!(result.is_err());

    // Test unsupported URI scheme
    let result = manager.read_resource("http://example.com/file.txt").await;
    assert!(result.is_err());
    
    Ok(())
}

/// Test resource annotations and metadata
#[tokio::test]
async fn test_resource_annotations() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    
    // Create a test file
    fs::write(temp_path.join("annotated.txt"), "Test content").await.unwrap();
    
    let provider = Arc::new(FileResourceProvider::new(temp_path, "file:".to_string())?);

    // List resources to get annotations
    let (resources, _) = provider.list_resources(None).await?;
    let resource = resources.iter().find(|r| r.name == "annotated.txt").unwrap();
    
    // Verify annotations are present
    assert!(resource.annotations.is_some());
    let annotations = resource.annotations.as_ref().unwrap();
    
    // Check size annotation
    assert_eq!(annotations.size, Some(12)); // "Test content" is 12 bytes
    
    // Check last modified is present
    assert!(annotations.last_modified.is_some());
    
    Ok(())
}

/// Test MIME type detection
#[tokio::test]
async fn test_mime_type_detection() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    
    // Create files with different extensions
    let test_files = vec![
        ("test.txt", "text/plain"),
        ("test.json", "application/json"),
        ("test.yaml", "application/yaml"),
        ("test.md", "text/markdown"),
        ("test.html", "text/html"),
        ("test.js", "application/javascript"),
        ("test.py", "text/x-python"),
        ("test.rs", "text/x-rust"),
    ];
    
    for (filename, _) in &test_files {
        fs::write(temp_path.join(filename), "content").await.unwrap();
    }
    
    let provider = Arc::new(FileResourceProvider::new(temp_path, "file:".to_string())?);
    let (resources, _) = provider.list_resources(None).await?;

    // Verify MIME types
    for (filename, expected_mime) in test_files {
        let resource = resources.iter().find(|r| r.name == filename).unwrap();
        assert_eq!(resource.mime_type, Some(expected_mime.to_string()), 
                  "MIME type mismatch for {}", filename);
    }
    
    Ok(())
}
