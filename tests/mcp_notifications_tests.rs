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
        // TODO: Change to true when resource list change notifications are implemented
        assert!(!default_caps.resources_list_changed);
        // TODO: Change to true when prompt list change notifications are implemented
        assert!(!default_caps.prompts_list_changed);
        // TODO: Change to true when resource subscriptions are fully exposed via MCP protocol
        assert!(!default_caps.resource_subscriptions);
        
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
        // TODO: Change to true when resource list change notifications are implemented
        assert!(!caps.resources_list_changed);
        // TODO: Change to true when prompt list change notifications are implemented
        assert!(!caps.prompts_list_changed);
        // TODO: Change to true when resource subscriptions are fully exposed via MCP protocol
        assert!(!caps.resource_subscriptions);
        
        // TODO: Update when resource subscriptions are implemented - currently returns Ok(empty_vec) even when capability disabled
        let subscriptions_result = manager.get_resource_subscriptions();
        assert!(subscriptions_result.is_ok(), "get_resource_subscriptions always succeeds");
        assert!(subscriptions_result.unwrap().is_empty(), "Should return empty list when no subscriptions");
    }

    #[tokio::test]
    async fn test_resources_list_changed_notification() {
        let manager = McpNotificationManager::new();
        let mut receiver = manager.subscribe();
        
        // TODO: Replace with actual notification test when resource list change notifications are implemented
        // Method succeeds but doesn't send notification when capability is disabled
        let result = manager.notify_resources_list_changed();
        assert!(result.is_ok(), "Method should succeed even when capability is disabled");
        
        // Should not receive any notification because capability is disabled
        let result = timeout(Duration::from_millis(100), receiver.recv()).await;
        assert!(result.is_err(), "Should not receive notification when capability is disabled");
    }

    #[tokio::test]
    async fn test_prompts_list_changed_notification() {
        let manager = McpNotificationManager::new();
        let mut receiver = manager.subscribe();
        
        // TODO: Replace with actual notification test when prompt list change notifications are implemented
        // Method succeeds but doesn't send notification when capability is disabled
        let result = manager.notify_prompts_list_changed();
        assert!(result.is_ok(), "Method should succeed even when capability is disabled");
        
        // Should not receive any notification because capability is disabled
        let result = timeout(Duration::from_millis(100), receiver.recv()).await;
        assert!(result.is_err(), "Should not receive notification when capability is disabled");
    }

    #[tokio::test]
    async fn test_resource_subscriptions() {
        let manager = McpNotificationManager::new();
        let test_uri = "file:///test/resource.txt".to_string();
        
        // TODO: Replace with actual resource subscription test when feature is implemented
        // For now, verify that resource subscriptions return errors when not implemented
        
        // Should return Ok(empty_vec) even when capability is disabled (backend exists)
        let result = manager.get_resource_subscriptions();
        assert!(result.is_ok(), "get_resource_subscriptions always succeeds");
        assert!(result.unwrap().is_empty(), "Should return empty list when no subscriptions");
        
        // Should return error when trying to subscribe
        let result = manager.subscribe_to_resource(test_uri.clone());
        assert!(result.is_err(), "subscribe_to_resource should return error when not implemented");
        
        // Should return error when trying to unsubscribe
        let result = manager.unsubscribe_from_resource(&test_uri);
        assert!(result.is_err(), "unsubscribe_from_resource should return error when not implemented");
    }

    #[tokio::test]
    async fn test_resource_updated_notification() {
        let manager = McpNotificationManager::new();
        let test_uri = "file:///test/resource.txt".to_string();
        
        // TODO: Replace with actual resource update notification test when feature is implemented
        // Method succeeds but doesn't send notification when capability is disabled
        let result = manager.notify_resource_updated(test_uri.clone());
        assert!(result.is_ok(), "Method should succeed even when capability is disabled");
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
        
        // TODO: Replace with actual stats test when resource subscriptions are implemented
        // For now, verify stats reflect unimplemented capabilities
        let stats = manager.get_stats().unwrap();
        assert_eq!(stats.resource_subscriptions_count, 0);
        // TODO: Change to true when resource list change notifications are implemented
        assert!(!stats.capabilities.resources_list_changed);
        
        // Resource subscription operations should fail when not implemented
        let result = manager.subscribe_to_resource("file:///test1.txt".to_string());
        assert!(result.is_err(), "subscribe_to_resource should fail when not implemented");
        let result = manager.subscribe_to_resource("file:///test2.txt".to_string());
        assert!(result.is_err(), "subscribe_to_resource should fail when not implemented");
        
        // Stats should still show 0 subscriptions
        let stats = manager.get_stats().unwrap();
        assert_eq!(stats.resource_subscriptions_count, 0);
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
        
        // TODO: Replace with actual multiple subscriber test when resource list change notifications are implemented
        // Method succeeds but doesn't send notification when capability is disabled
        let result = manager.notify_resources_list_changed();
        assert!(result.is_ok(), "Method should succeed even when capability is disabled");
        
        // Neither subscriber should receive notification when capability is disabled
        let result1 = timeout(Duration::from_millis(100), receiver1.recv()).await;
        assert!(result1.is_err(), "Should not receive notification when capability is disabled");
        
        let result2 = timeout(Duration::from_millis(100), receiver2.recv()).await;
        assert!(result2.is_err(), "Should not receive notification when capability is disabled");
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
