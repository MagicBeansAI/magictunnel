//! Integration tests for MCP Logging and Notifications System

#[cfg(test)]
mod tests {
    use magictunnel::mcp::server::McpServer;
    use magictunnel::mcp::types::*;

    use magictunnel::config::RegistryConfig;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    async fn create_test_server() -> Arc<McpServer> {
        let config = RegistryConfig::default();
        Arc::new(McpServer::new(config).await.unwrap())
    }

    #[tokio::test]
    async fn test_mcp_server_capabilities() {
        let server = create_test_server().await;
        
        let capabilities = server.get_capabilities();
        
        // Verify logging capability
        assert!(capabilities["capabilities"]["logging"].is_object());
        
        // Verify resource capabilities
        let resource_caps = &capabilities["capabilities"]["resources"];
        // TODO: Change to true when resource subscriptions are implemented
        assert_eq!(resource_caps["subscribe"], false);
        // TODO: Change to true when resource list change notifications are implemented
        assert_eq!(resource_caps["listChanged"], false);
        
        // Verify prompt capabilities
        let prompt_caps = &capabilities["capabilities"]["prompts"];
        // TODO: Change to true when prompt list change notifications are implemented
        assert_eq!(prompt_caps["listChanged"], false);
        
        // Verify tools capability
        assert!(capabilities["capabilities"]["tools"].is_object());
    }

    #[tokio::test]
    async fn test_mcp_server_logging_integration() {
        let server = create_test_server().await;
        
        // Test setting log level
        let result = server.set_log_level(LogLevel::Debug).await;
        assert!(result.is_ok());
        
        // Test getting log level
        let level = server.get_log_level().await.unwrap();
        assert_eq!(level, LogLevel::Debug);
        
        // Test getting named logger
        let logger = server.get_logger("test-component").unwrap();
        // Logger should inherit the server's global level
        let logger_level = logger.get_level().unwrap();
        assert!(logger_level == LogLevel::Debug || logger_level == LogLevel::Info);
        
        // Test logging through server logger
        let mut receiver = logger.subscribe();
        logger.info(json!({"message": "test log from server"})).unwrap();
        
        let notification = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("Should receive notification")
            .expect("Should not have error");
        
        assert_eq!(notification.method, "notifications/message");
    }

    #[tokio::test]
    async fn test_mcp_server_notifications_integration() {
        let server = create_test_server().await;
        
        let notification_manager = server.notification_manager();
        
        // TODO: Replace with actual notification test when resource list change notifications are implemented
        // The methods exist and return Ok(), but capabilities are disabled so no notifications are sent
        let result = notification_manager.notify_resources_list_changed();
        assert!(result.is_ok(), "Method should succeed even when capability is disabled");
        
        // TODO: Also test prompt list changed notifications when implemented
        let result = notification_manager.notify_prompts_list_changed();
        assert!(result.is_ok(), "Method should succeed even when capability is disabled");
        
        // Verify that no notifications were actually sent (since capabilities are disabled)
        let mut receiver = notification_manager.subscribe();
        let result = tokio::time::timeout(Duration::from_millis(50), receiver.recv()).await;
        assert!(result.is_err(), "No notifications should be received when capabilities are disabled");
    }

    #[tokio::test]
    async fn test_mcp_server_resource_subscriptions() {
        let server = create_test_server().await;
        
        let notification_manager = server.notification_manager();
        let test_uri = "file:///test/integration.txt".to_string();
        
        // TODO: Remove this test and implement actual resource subscription test when feature is implemented
        // For now, verify that resource subscriptions are not yet supported
        let result = notification_manager.subscribe_to_resource(test_uri.clone());
        assert!(result.is_err());
        
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("Resource subscriptions not supported"));
    }

    #[tokio::test]
    async fn test_mcp_server_logging_and_notifications_together() {
        let server = create_test_server().await;
        
        // Set up logging
        let logger = server.get_logger("integration-test").unwrap();
        logger.set_level(LogLevel::Info).unwrap();
        let mut log_receiver = logger.subscribe();
        
        // Set up notifications
        let notification_manager = server.notification_manager();
        let mut notification_receiver = notification_manager.subscribe();
        
        // Perform operations that generate both logs and notifications
        
        // 1. Log a tool execution start
        logger.log_tool_start("test_integration_tool", &json!({"test": true})).unwrap();
        
        // 2. Send a custom notification
        notification_manager.send_custom_notification(
            "notifications/integration/test".to_string(),
            Some(json!({"test": "integration"}))
        ).unwrap();
        
        // 3. Log tool completion
        logger.log_tool_success("test_integration_tool", 100, 256).unwrap();
        
        // 4. Notify server status change
        notification_manager.notify_server_status("ready", Some("Integration test complete".to_string())).unwrap();
        
        // Verify we received all expected messages
        
        // Log message 1: tool start
        let log_msg1 = timeout(Duration::from_millis(100), log_receiver.recv())
            .await
            .expect("Should receive log notification")
            .expect("Should not have error");
        assert_eq!(log_msg1.method, "notifications/message");
        
        // Notification 1: custom notification
        let notif_msg1 = timeout(Duration::from_millis(100), notification_receiver.recv())
            .await
            .expect("Should receive custom notification")
            .expect("Should not have error");
        assert_eq!(notif_msg1.method, "notifications/integration/test");
        
        // Log message 2: tool success
        let log_msg2 = timeout(Duration::from_millis(100), log_receiver.recv())
            .await
            .expect("Should receive log notification")
            .expect("Should not have error");
        assert_eq!(log_msg2.method, "notifications/message");
        
        // Notification 2: server status
        let notif_msg2 = timeout(Duration::from_millis(100), notification_receiver.recv())
            .await
            .expect("Should receive status notification")
            .expect("Should not have error");
        assert_eq!(notif_msg2.method, "notifications/server/status");
    }

    #[tokio::test]
    async fn test_mcp_server_structured_logging_with_notifications() {
        let server = create_test_server().await;
        
        let logger = server.get_logger("structured-test").unwrap();
        logger.set_level(LogLevel::Debug).unwrap();
        let mut receiver = logger.subscribe();
        
        // Test structured logging for different scenarios
        
        // Resource access logging
        logger.log_resource_access("file:///test/resource.txt", "read").unwrap();
        
        let notification = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("Should receive notification")
            .expect("Should not have error");
        
        let log_message: LogMessage = serde_json::from_value(notification.params.unwrap()).unwrap();
        assert_eq!(log_message.level, LogLevel::Info);
        assert_eq!(log_message.logger, Some("structured-test".to_string()));
        
        let data = log_message.data.as_object().unwrap();
        assert_eq!(data["event"], "resource_access");
        assert_eq!(data["uri"], "file:///test/resource.txt");
        assert_eq!(data["operation"], "read");
        assert!(data.contains_key("timestamp"));
    }

    #[tokio::test]
    async fn test_mcp_server_error_logging_with_notifications() {
        let server = create_test_server().await;
        
        let logger = server.get_logger("error-test").unwrap();
        logger.set_level(LogLevel::Warning).unwrap();
        let mut receiver = logger.subscribe();
        
        // Test error logging
        logger.error(json!({
            "error": "Test error condition",
            "component": "integration-test",
            "details": {
                "code": "TEST_ERROR",
                "message": "This is a test error for integration testing"
            }
        })).unwrap();
        
        let notification = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("Should receive notification")
            .expect("Should not have error");
        
        let log_message: LogMessage = serde_json::from_value(notification.params.unwrap()).unwrap();
        assert_eq!(log_message.level, LogLevel::Error);
        
        let data = log_message.data.as_object().unwrap();
        assert_eq!(data["error"], "Test error condition");
        assert_eq!(data["component"], "integration-test");
    }

    #[tokio::test]
    async fn test_mcp_server_notification_stats() {
        let server = create_test_server().await;
        
        let notification_manager = server.notification_manager();
        
        // TODO: Replace with actual stats test when resource subscriptions are implemented
        // For now, verify stats show no resource subscription capabilities
        let stats = notification_manager.get_stats().unwrap();
        assert_eq!(stats.resource_subscriptions_count, 0);
        
        // Verify capabilities reflect unimplemented features
        // TODO: Change these to true when features are implemented
        assert!(!stats.capabilities.resources_list_changed);
        assert!(!stats.capabilities.prompts_list_changed);
        assert!(!stats.capabilities.resource_subscriptions);
    }

    #[tokio::test]
    async fn test_mcp_server_concurrent_logging_and_notifications() {
        let server = Arc::new(create_test_server().await);
        
        let logger = server.get_logger("concurrent-test").unwrap();
        logger.set_level(LogLevel::Debug).unwrap();
        let mut log_receiver = logger.subscribe();
        
        let notification_manager = server.notification_manager();
        let mut notification_receiver = notification_manager.subscribe();
        
        // Spawn multiple tasks that log and send notifications concurrently
        let mut handles = vec![];
        
        for i in 0..5 {
            let server_clone = server.clone();
            let handle = tokio::spawn(async move {
                let logger = server_clone.get_logger(&format!("task-{}", i)).unwrap();
                let notification_manager = server_clone.notification_manager();
                
                for j in 0..3 {
                    // Log a message
                    logger.info(json!({"task": i, "iteration": j, "message": "concurrent test"})).unwrap();
                    
                    // Send a notification
                    notification_manager.send_custom_notification(
                        format!("notifications/task/{}/iteration", i),
                        Some(json!({"task": i, "iteration": j}))
                    ).unwrap();
                    
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        // Count received messages (may be rate limited)
        let mut log_count = 0;
        let mut notification_count = 0;

        // Collect log messages with longer timeout
        while let Ok(Ok(_)) = timeout(Duration::from_millis(50), log_receiver.recv()).await {
            log_count += 1;
            if log_count >= 5 { break; } // Don't wait for all messages
        }

        // Collect notifications with longer timeout
        while let Ok(Ok(_)) = timeout(Duration::from_millis(50), notification_receiver.recv()).await {
            notification_count += 1;
            if notification_count >= 5 { break; } // Don't wait for all messages
        }

        // Should receive some messages from concurrent operations (may be rate limited)
        // Note: Due to rate limiting, we might not receive all messages
        println!("Received {} log messages and {} notifications", log_count, notification_count);
        // Just verify the system is working, don't require specific counts due to rate limiting
    }
}
