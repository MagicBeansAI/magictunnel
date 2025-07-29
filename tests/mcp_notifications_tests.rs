//! Tests for MCP Notifications System

#[cfg(test)]
mod tests {
    use magictunnel::mcp::notifications::*;
    use magictunnel::mcp::types::*;
    use serde_json::json;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_notification_capabilities() {
        let default_caps = NotificationCapabilities::default();
        assert!(default_caps.resources_list_changed);
        assert!(default_caps.prompts_list_changed);
        assert!(default_caps.resource_subscriptions);
        
        let custom_caps = NotificationCapabilities {
            resources_list_changed: false,
            prompts_list_changed: true,
            tools_list_changed: true,
            resource_subscriptions: false,
        };
        
        let manager = McpNotificationManager::with_capabilities(custom_caps.clone());
        let caps = manager.capabilities();
        assert!(!caps.resources_list_changed);
        assert!(caps.prompts_list_changed);
        assert!(!caps.resource_subscriptions);
    }

    #[tokio::test]
    async fn test_notification_manager_creation() {
        let manager = McpNotificationManager::new();
        let caps = manager.capabilities();
        assert!(caps.resources_list_changed);
        assert!(caps.prompts_list_changed);
        assert!(caps.resource_subscriptions);
        
        let subscriptions = manager.get_resource_subscriptions().unwrap();
        assert!(subscriptions.is_empty());
    }

    #[tokio::test]
    async fn test_resources_list_changed_notification() {
        let manager = McpNotificationManager::new();
        let mut receiver = manager.subscribe();
        
        manager.notify_resources_list_changed().unwrap();
        
        let notification = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("Should receive notification")
            .expect("Should not have error");
        
        assert_eq!(notification.method, "notifications/resources/list_changed");
        assert!(notification.params.is_none());
    }

    #[tokio::test]
    async fn test_prompts_list_changed_notification() {
        let manager = McpNotificationManager::new();
        let mut receiver = manager.subscribe();
        
        manager.notify_prompts_list_changed().unwrap();
        
        let notification = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("Should receive notification")
            .expect("Should not have error");
        
        assert_eq!(notification.method, "notifications/prompts/list_changed");
        assert!(notification.params.is_none());
    }

    #[tokio::test]
    async fn test_resource_subscriptions() {
        let manager = McpNotificationManager::new();
        let test_uri = "file:///test/resource.txt".to_string();
        
        // Initially no subscriptions
        let subscriptions = manager.get_resource_subscriptions().unwrap();
        assert!(subscriptions.is_empty());
        
        // Subscribe to resource
        manager.subscribe_to_resource(test_uri.clone()).unwrap();
        
        let subscriptions = manager.get_resource_subscriptions().unwrap();
        assert_eq!(subscriptions.len(), 1);
        assert!(subscriptions.contains(&test_uri));
        
        // Subscribe to same resource again (should be idempotent)
        manager.subscribe_to_resource(test_uri.clone()).unwrap();
        
        let subscriptions = manager.get_resource_subscriptions().unwrap();
        assert_eq!(subscriptions.len(), 1);
        
        // Unsubscribe from resource
        manager.unsubscribe_from_resource(&test_uri).unwrap();
        
        let subscriptions = manager.get_resource_subscriptions().unwrap();
        assert!(subscriptions.is_empty());
    }

    #[tokio::test]
    async fn test_resource_updated_notification() {
        let manager = McpNotificationManager::new();
        let mut receiver = manager.subscribe();
        let test_uri = "file:///test/resource.txt".to_string();
        
        // Subscribe to resource first
        manager.subscribe_to_resource(test_uri.clone()).unwrap();
        
        // Notify resource updated
        manager.notify_resource_updated(test_uri.clone()).unwrap();
        
        let notification = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("Should receive notification")
            .expect("Should not have error");
        
        assert_eq!(notification.method, "notifications/resources/updated");
        assert!(notification.params.is_some());
        
        let params = notification.params.unwrap();
        assert_eq!(params["uri"], test_uri);
    }

    #[tokio::test]
    async fn test_resource_updated_no_subscription() {
        let manager = McpNotificationManager::new();
        let mut receiver = manager.subscribe();
        let test_uri = "file:///test/resource.txt".to_string();
        
        // Don't subscribe to resource
        
        // Notify resource updated
        manager.notify_resource_updated(test_uri.clone()).unwrap();
        
        // Should not receive notification
        let result = timeout(Duration::from_millis(50), receiver.recv()).await;
        assert!(result.is_err(), "Should not receive notification for unsubscribed resource");
    }

    #[tokio::test]
    async fn test_custom_notifications() {
        let manager = McpNotificationManager::new();
        let mut receiver = manager.subscribe();
        
        let custom_method = "notifications/custom/test".to_string();
        let custom_params = json!({"test": "data", "value": 42});
        
        manager.send_custom_notification(custom_method.clone(), Some(custom_params.clone())).unwrap();
        
        let notification = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("Should receive notification")
            .expect("Should not have error");
        
        assert_eq!(notification.method, custom_method);
        assert_eq!(notification.params, Some(custom_params));
    }

    #[tokio::test]
    async fn test_tool_execution_notifications() {
        let manager = McpNotificationManager::new();
        let mut receiver = manager.subscribe();
        
        let tool_name = "test_tool";
        let event = "started";
        let data = json!({"arguments": {"arg1": "value1"}});
        
        manager.notify_tool_execution(tool_name, event, data.clone()).unwrap();
        
        let notification = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("Should receive notification")
            .expect("Should not have error");
        
        assert_eq!(notification.method, "notifications/tools/started");
        assert!(notification.params.is_some());
        
        let params = notification.params.unwrap();
        assert_eq!(params["tool_name"], tool_name);
        assert_eq!(params["event"], event);
        assert_eq!(params["data"], data);
        assert!(params["timestamp"].is_string());
    }

    #[tokio::test]
    async fn test_server_status_notifications() {
        let manager = McpNotificationManager::new();
        let mut receiver = manager.subscribe();
        
        let status = "ready";
        let message = Some("Server is ready to accept requests".to_string());
        
        manager.notify_server_status(status, message.clone()).unwrap();
        
        let notification = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("Should receive notification")
            .expect("Should not have error");
        
        assert_eq!(notification.method, "notifications/server/status");
        assert!(notification.params.is_some());
        
        let params = notification.params.unwrap();
        assert_eq!(params["status"], status);
        assert_eq!(params["message"], message.unwrap());
        assert!(params["timestamp"].is_string());
    }

    #[tokio::test]
    async fn test_capabilities_changed_notifications() {
        let manager = McpNotificationManager::new();
        let mut receiver = manager.subscribe();
        
        let capabilities = json!({
            "logging": {},
            "resources": {"subscribe": true, "listChanged": true}
        });
        
        manager.notify_capabilities_changed(capabilities.clone()).unwrap();
        
        let notification = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("Should receive notification")
            .expect("Should not have error");
        
        assert_eq!(notification.method, "notifications/server/capabilities_changed");
        assert!(notification.params.is_some());
        
        let params = notification.params.unwrap();
        assert_eq!(params["capabilities"], capabilities);
        assert!(params["timestamp"].is_string());
    }

    #[tokio::test]
    async fn test_notification_stats() {
        let manager = McpNotificationManager::new();
        
        // Initially no subscriptions
        let stats = manager.get_stats().unwrap();
        assert_eq!(stats.resource_subscriptions_count, 0);
        assert!(stats.capabilities.resources_list_changed);
        
        // Add some subscriptions
        manager.subscribe_to_resource("file:///test1.txt".to_string()).unwrap();
        manager.subscribe_to_resource("file:///test2.txt".to_string()).unwrap();
        
        let stats = manager.get_stats().unwrap();
        assert_eq!(stats.resource_subscriptions_count, 2);
    }

    #[tokio::test]
    async fn test_disabled_capabilities() {
        let caps = NotificationCapabilities {
            resources_list_changed: false,
            prompts_list_changed: false,
            tools_list_changed: false,
            resource_subscriptions: false,
        };
        
        let manager = McpNotificationManager::with_capabilities(caps);
        let mut receiver = manager.subscribe();
        
        // Try to send notifications that are disabled
        manager.notify_resources_list_changed().unwrap();
        manager.notify_prompts_list_changed().unwrap();
        
        // Should not receive any notifications
        let result = timeout(Duration::from_millis(50), receiver.recv()).await;
        assert!(result.is_err(), "Should not receive notifications when capabilities are disabled");
        
        // Resource subscriptions should fail
        let result = manager.subscribe_to_resource("file:///test.txt".to_string());
        assert!(result.is_err(), "Resource subscriptions should fail when disabled");
    }

    #[tokio::test]
    async fn test_notification_event_types() {
        let event = NotificationEvent::ResourcesListChanged;
        assert_eq!(event.event_type(), "resources_list_changed");
        
        let event = NotificationEvent::ResourceUpdated { uri: "test".to_string() };
        assert_eq!(event.event_type(), "resource_updated");
        
        let event = NotificationEvent::ToolExecutionStarted { tool_name: "test".to_string() };
        assert_eq!(event.event_type(), "tool_execution_started");
        
        let event = NotificationEvent::Custom { method: "test".to_string() };
        assert_eq!(event.event_type(), "custom");
    }

    #[tokio::test]
    async fn test_notification_event_json() {
        let event = NotificationEvent::ResourceUpdated { uri: "file:///test.txt".to_string() };
        let json_value = event.to_json();
        
        assert_eq!(json_value["event"], "resource_updated");
        assert_eq!(json_value["uri"], "file:///test.txt");
        
        let event = NotificationEvent::ToolExecutionCompleted { 
            tool_name: "test_tool".to_string(), 
            success: true 
        };
        let json_value = event.to_json();
        
        assert_eq!(json_value["event"], "tool_execution_completed");
        assert_eq!(json_value["tool_name"], "test_tool");
        assert_eq!(json_value["success"], true);
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let manager = McpNotificationManager::new();
        let mut receiver1 = manager.subscribe();
        let mut receiver2 = manager.subscribe();
        
        manager.notify_resources_list_changed().unwrap();
        
        // Both subscribers should receive the notification
        let notification1 = timeout(Duration::from_millis(100), receiver1.recv())
            .await
            .expect("Should receive notification")
            .expect("Should not have error");
        
        let notification2 = timeout(Duration::from_millis(100), receiver2.recv())
            .await
            .expect("Should receive notification")
            .expect("Should not have error");
        
        assert_eq!(notification1.method, "notifications/resources/list_changed");
        assert_eq!(notification2.method, "notifications/resources/list_changed");
    }

    #[tokio::test]
    async fn test_mcp_notification_creation() {
        let notification = McpNotification::new("test/method".to_string());
        assert_eq!(notification.method, "test/method");
        assert!(notification.params.is_none());
        
        let params = json!({"test": "data"});
        let notification = McpNotification::with_params("test/method".to_string(), params.clone());
        assert_eq!(notification.method, "test/method");
        assert_eq!(notification.params, Some(params));
        
        let notification = McpNotification::resources_list_changed();
        assert_eq!(notification.method, "notifications/resources/list_changed");
        
        let notification = McpNotification::prompts_list_changed();
        assert_eq!(notification.method, "notifications/prompts/list_changed");
        
        let notification = McpNotification::resource_updated("file:///test.txt".to_string());
        assert_eq!(notification.method, "notifications/resources/updated");
        assert!(notification.params.is_some());
    }
}
