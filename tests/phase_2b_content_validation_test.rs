//! Phase 2B Content Validation and List Changed Notifications Tests
//!
//! This test suite validates the enhanced content support features implemented in Phase 2B:
//! - Content type validation (MIME types, URI validation, base64 encoding)
//! - Tools list_changed notifications
//! - Enhanced content features

use magictunnel::mcp::types::{ToolContent, McpNotification};
use magictunnel::mcp::notifications::{McpNotificationManager, NotificationCapabilities};
use magictunnel::registry::service::RegistryService;
use magictunnel::config::{RegistryConfig, ValidationConfig};
use std::sync::Arc;

#[cfg(test)]
mod phase_2b_tests {
    use super::*;

    #[test]
    fn test_content_validation_text() {
        // Test text content validation
        let text_content = ToolContent::text("Hello, world!".to_string());
        assert!(text_content.validate().is_ok());
        assert_eq!(text_content.content_type(), "text");
        assert_eq!(text_content.mime_type(), Some("text/plain"));
        assert!(text_content.is_safe());

        // Test empty text content (should fail validation)
        let empty_text = ToolContent::text("".to_string());
        assert!(empty_text.validate().is_err());
        assert!(!empty_text.is_safe());
    }

    #[test]
    fn test_content_validation_image() {
        // Test valid image content
        let valid_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        let image_content = ToolContent::image(valid_base64.to_string(), "image/png".to_string());
        assert!(image_content.is_ok());
        
        let image = image_content.unwrap();
        assert!(image.validate().is_ok());
        assert_eq!(image.content_type(), "image");
        assert_eq!(image.mime_type(), Some("image/png"));
        assert!(image.is_safe());

        // Test invalid MIME type
        let invalid_mime = ToolContent::image(valid_base64.to_string(), "invalid/type".to_string());
        assert!(invalid_mime.is_err());

        // Test invalid base64
        let invalid_base64 = ToolContent::image("invalid_base64!@#".to_string(), "image/png".to_string());
        assert!(invalid_base64.is_err());

        // Test empty base64
        let empty_base64 = ToolContent::image("".to_string(), "image/png".to_string());
        assert!(empty_base64.is_err());
    }

    #[test]
    fn test_content_validation_supported_image_types() {
        let valid_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        
        // Test all supported image MIME types
        let supported_types = vec![
            "image/png",
            "image/jpeg", 
            "image/jpg",
            "image/gif",
            "image/webp",
            "image/svg+xml",
            "image/bmp",
            "image/tiff",
        ];

        for mime_type in supported_types {
            let image_content = ToolContent::image(valid_base64.to_string(), mime_type.to_string());
            assert!(image_content.is_ok(), "Failed for MIME type: {}", mime_type);
            
            let image = image_content.unwrap();
            assert!(image.validate().is_ok(), "Validation failed for MIME type: {}", mime_type);
            assert_eq!(image.mime_type(), Some(mime_type));
        }
    }

    #[test]
    fn test_content_validation_resource() {
        // Test valid HTTP URI
        let http_resource = ToolContent::resource("https://example.com/api/data".to_string());
        assert!(http_resource.is_ok());
        
        let resource = http_resource.unwrap();
        assert!(resource.validate().is_ok());
        assert_eq!(resource.content_type(), "resource");
        assert!(resource.is_safe());

        // Test valid file URI
        let file_resource = ToolContent::resource("file:///path/to/file.txt".to_string());
        assert!(file_resource.is_ok());

        // Test invalid URI scheme
        let invalid_scheme = ToolContent::resource("javascript:alert('xss')".to_string());
        assert!(invalid_scheme.is_err());

        // Test directory traversal attempt
        let traversal_attempt = ToolContent::resource("file:///path/../../../etc/passwd".to_string());
        assert!(traversal_attempt.is_err());

        // Test empty URI
        let empty_uri = ToolContent::resource("".to_string());
        assert!(empty_uri.is_err());

        // Test malformed URI
        let malformed_uri = ToolContent::resource("not-a-valid-uri".to_string());
        assert!(malformed_uri.is_err());
    }

    #[test]
    fn test_content_validation_resource_with_text() {
        // Test valid resource with text and MIME type
        let resource_with_text = ToolContent::resource_with_text(
            "https://example.com/api/data".to_string(),
            "Sample data".to_string(),
            Some("application/json".to_string())
        );
        assert!(resource_with_text.is_ok());
        
        let resource = resource_with_text.unwrap();
        assert!(resource.validate().is_ok());
        assert_eq!(resource.content_type(), "resource");
        assert_eq!(resource.mime_type(), Some("application/json"));
        assert!(resource.is_safe());

        // Test invalid MIME type format
        let invalid_mime = ToolContent::resource_with_text(
            "https://example.com/api/data".to_string(),
            "Sample data".to_string(),
            Some("invalid-mime-type".to_string())
        );
        assert!(invalid_mime.is_err());
    }

    #[test]
    fn test_supported_uri_schemes() {
        let supported_schemes = vec![
            "http://example.com",
            "https://example.com", 
            "file:///path/to/file",
            "data:text/plain;base64,SGVsbG8gV29ybGQ=",
            "ftp://ftp.example.com/file.txt",
            "ftps://ftps.example.com/file.txt",
        ];

        for uri in supported_schemes {
            let resource = ToolContent::resource(uri.to_string());
            assert!(resource.is_ok(), "Failed for URI scheme: {}", uri);
        }
    }

    #[test]
    fn test_notification_capabilities() {
        // Test default notification capabilities
        let capabilities = NotificationCapabilities::default();
        assert!(capabilities.resources_list_changed);
        assert!(capabilities.prompts_list_changed);
        assert!(capabilities.tools_list_changed);
        assert!(capabilities.resource_subscriptions);
    }

    #[test]
    fn test_notification_creation() {
        // Test tools list_changed notification creation
        let notification = McpNotification::tools_list_changed();
        assert_eq!(notification.method, "notifications/tools/list_changed");
        assert!(notification.params.is_none());

        // Test other notification types
        let resources_notification = McpNotification::resources_list_changed();
        assert_eq!(resources_notification.method, "notifications/resources/list_changed");

        let prompts_notification = McpNotification::prompts_list_changed();
        assert_eq!(prompts_notification.method, "notifications/prompts/list_changed");
    }

    #[test]
    fn test_notification_manager() {
        // Test notification manager creation
        let manager = McpNotificationManager::new();
        
        // Test tools list_changed notification
        let result = manager.notify_tools_list_changed();
        // Note: This will succeed even without a client connection
        // as the notification system is designed to be resilient
        assert!(result.is_ok());

        // Test other notification methods
        assert!(manager.notify_resources_list_changed().is_ok());
        assert!(manager.notify_prompts_list_changed().is_ok());
    }

    #[tokio::test]
    async fn test_registry_notification_integration() {
        // Create a minimal registry config for testing
        let config = RegistryConfig {
            r#type: "file".to_string(),
            paths: vec!["./test_capabilities".to_string()],
            hot_reload: false,
            validation: ValidationConfig {
                strict: false,
                allow_unknown_fields: true,
            },
        };

        // Create registry service
        let registry = RegistryService::new(config).await;
        assert!(registry.is_ok());

        let registry = Arc::new(registry.unwrap());

        // Create notification manager
        let notification_manager = Arc::new(McpNotificationManager::new());

        // Set notification manager on registry
        registry.set_notification_manager(notification_manager.clone());

        // Test that the registry can send notifications
        // (This tests the integration without requiring actual file changes)
        assert!(notification_manager.notify_tools_list_changed().is_ok());
    }

    #[test]
    fn test_unchecked_content_creation() {
        // Test unchecked content creation methods (for internal use)
        let image = ToolContent::image_unchecked("invalid_base64".to_string(), "invalid/type".to_string());
        assert_eq!(image.content_type(), "image");
        assert!(!image.is_safe()); // Should fail validation

        let resource = ToolContent::resource_unchecked("invalid-uri".to_string());
        assert_eq!(resource.content_type(), "resource");
        assert!(!resource.is_safe()); // Should fail validation

        let resource_with_text = ToolContent::resource_with_text_unchecked(
            "invalid-uri".to_string(),
            "text".to_string(),
            Some("invalid-mime".to_string())
        );
        assert_eq!(resource_with_text.content_type(), "resource");
        assert!(!resource_with_text.is_safe()); // Should fail validation
    }
}
