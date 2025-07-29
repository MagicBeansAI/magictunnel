//! Tests for MCP Logging System

#[cfg(test)]
mod tests {
    use magictunnel::mcp::logging::*;
    use magictunnel::mcp::types::*;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_log_level_parsing() {
        assert_eq!(LogLevel::from_str("debug").unwrap(), LogLevel::Debug);
        assert_eq!(LogLevel::from_str("info").unwrap(), LogLevel::Info);
        assert_eq!(LogLevel::from_str("warning").unwrap(), LogLevel::Warning);
        assert_eq!(LogLevel::from_str("warn").unwrap(), LogLevel::Warning);
        assert_eq!(LogLevel::from_str("error").unwrap(), LogLevel::Error);
        assert_eq!(LogLevel::from_str("critical").unwrap(), LogLevel::Critical);
        assert_eq!(LogLevel::from_str("alert").unwrap(), LogLevel::Alert);
        assert_eq!(LogLevel::from_str("emergency").unwrap(), LogLevel::Emergency);
        
        assert!(LogLevel::from_str("invalid").is_err());
    }

    #[tokio::test]
    async fn test_log_level_ordering() {
        // Test numeric ordering (lower numbers = higher severity in syslog)
        assert!(LogLevel::Emergency.to_numeric() < LogLevel::Alert.to_numeric());
        assert!(LogLevel::Alert.to_numeric() < LogLevel::Critical.to_numeric());
        assert!(LogLevel::Critical.to_numeric() < LogLevel::Error.to_numeric());
        assert!(LogLevel::Error.to_numeric() < LogLevel::Warning.to_numeric());
        assert!(LogLevel::Warning.to_numeric() < LogLevel::Notice.to_numeric());
        assert!(LogLevel::Notice.to_numeric() < LogLevel::Info.to_numeric());
        assert!(LogLevel::Info.to_numeric() < LogLevel::Debug.to_numeric());
    }

    #[tokio::test]
    async fn test_log_level_should_log() {
        let min_level = LogLevel::Warning;
        
        assert!(LogLevel::Emergency.should_log(min_level));
        assert!(LogLevel::Alert.should_log(min_level));
        assert!(LogLevel::Critical.should_log(min_level));
        assert!(LogLevel::Error.should_log(min_level));
        assert!(LogLevel::Warning.should_log(min_level));
        assert!(!LogLevel::Notice.should_log(min_level));
        assert!(!LogLevel::Info.should_log(min_level));
        assert!(!LogLevel::Debug.should_log(min_level));
    }

    #[tokio::test]
    async fn test_mcp_logger_creation() {
        let logger = McpLogger::new();
        assert_eq!(logger.get_level().unwrap(), LogLevel::Info);
        
        let named_logger = McpLogger::with_name("test-logger".to_string());
        assert_eq!(named_logger.get_level().unwrap(), LogLevel::Info);
    }

    #[tokio::test]
    async fn test_mcp_logger_set_level() {
        let logger = McpLogger::new();
        
        logger.set_level(LogLevel::Debug).unwrap();
        assert_eq!(logger.get_level().unwrap(), LogLevel::Debug);
        
        logger.set_level(LogLevel::Error).unwrap();
        assert_eq!(logger.get_level().unwrap(), LogLevel::Error);
    }

    #[tokio::test]
    async fn test_mcp_logger_logging() {
        let logger = McpLogger::new();
        logger.set_level(LogLevel::Debug).unwrap();
        
        // Subscribe to notifications
        let mut receiver = logger.subscribe();
        
        // Log a message
        let test_data = json!({"message": "test log message", "component": "test"});
        logger.info(test_data.clone()).unwrap();
        
        // Check notification was sent
        let notification = tokio::time::timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("Should receive notification")
            .expect("Should not have error");
        
        assert_eq!(notification.method, "notifications/message");
        assert!(notification.params.is_some());
        
        let log_message: LogMessage = serde_json::from_value(notification.params.unwrap()).unwrap();
        assert_eq!(log_message.level, LogLevel::Info);
        assert_eq!(log_message.data, test_data);
    }

    #[tokio::test]
    async fn test_mcp_logger_level_filtering() {
        let logger = McpLogger::new();
        logger.set_level(LogLevel::Warning).unwrap();
        
        let mut receiver = logger.subscribe();
        
        // Log below minimum level - should not send notification
        logger.info(json!({"message": "info message"})).unwrap();
        
        // Check no notification was sent
        let result = tokio::time::timeout(Duration::from_millis(50), receiver.recv()).await;
        assert!(result.is_err(), "Should not receive notification for filtered message");
        
        // Log at minimum level - should send notification
        logger.warning(json!({"message": "warning message"})).unwrap();
        
        // Check notification was sent
        let notification = tokio::time::timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("Should receive notification")
            .expect("Should not have error");
        
        assert_eq!(notification.method, "notifications/message");
    }

    #[tokio::test]
    async fn test_mcp_logger_convenience_methods() {
        let logger = McpLogger::new();
        logger.set_level(LogLevel::Debug).unwrap();
        
        let mut receiver = logger.subscribe();
        
        // Test all convenience methods
        logger.debug(json!({"level": "debug"})).unwrap();
        logger.info(json!({"level": "info"})).unwrap();
        logger.notice(json!({"level": "notice"})).unwrap();
        logger.warning(json!({"level": "warning"})).unwrap();
        logger.error(json!({"level": "error"})).unwrap();
        logger.critical(json!({"level": "critical"})).unwrap();
        logger.alert(json!({"level": "alert"})).unwrap();
        logger.emergency(json!({"level": "emergency"})).unwrap();
        
        // Should receive 8 notifications
        for expected_level in [
            LogLevel::Debug, LogLevel::Info, LogLevel::Notice, LogLevel::Warning,
            LogLevel::Error, LogLevel::Critical, LogLevel::Alert, LogLevel::Emergency
        ] {
            let notification = tokio::time::timeout(Duration::from_millis(100), receiver.recv())
                .await
                .expect("Should receive notification")
                .expect("Should not have error");
            
            let log_message: LogMessage = serde_json::from_value(notification.params.unwrap()).unwrap();
            assert_eq!(log_message.level, expected_level);
        }
    }

    #[tokio::test]
    async fn test_mcp_logger_structured_logging() {
        let logger = McpLogger::with_name("test-component".to_string());
        logger.set_level(LogLevel::Debug).unwrap();
        
        let mut receiver = logger.subscribe();
        
        // Test tool execution logging
        logger.log_tool_start("test_tool", &json!({"arg1": "value1"})).unwrap();
        
        let notification = tokio::time::timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("Should receive notification")
            .expect("Should not have error");
        
        let log_message: LogMessage = serde_json::from_value(notification.params.unwrap()).unwrap();
        assert_eq!(log_message.level, LogLevel::Info);
        assert_eq!(log_message.logger, Some("test-component".to_string()));
        
        let data = log_message.data.as_object().unwrap();
        assert_eq!(data["event"], "tool_execution_start");
        assert_eq!(data["tool_name"], "test_tool");
    }

    #[tokio::test]
    async fn test_mcp_logger_manager() {
        let manager = McpLoggerManager::new();
        
        // Test default logger
        let default_logger = manager.default_logger();
        assert_eq!(default_logger.get_level().unwrap(), LogLevel::Info);
        
        // Test named logger creation
        let named_logger = manager.get_logger("test-logger").unwrap();
        assert_eq!(named_logger.get_level().unwrap(), LogLevel::Info);
        
        // Test getting same logger returns same instance
        let same_logger = manager.get_logger("test-logger").unwrap();
        assert!(Arc::ptr_eq(&named_logger, &same_logger));
        
        // Test global level setting
        manager.set_global_level(LogLevel::Debug).unwrap();
        assert_eq!(default_logger.get_level().unwrap(), LogLevel::Debug);
        assert_eq!(named_logger.get_level().unwrap(), LogLevel::Debug);
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let logger = McpLogger::new();
        logger.set_level(LogLevel::Debug).unwrap();
        
        let mut receiver = logger.subscribe();
        
        // Send many messages quickly
        for i in 0..150 {
            logger.info(json!({"message": format!("Message {}", i)})).unwrap();
        }
        
        // Count received notifications
        let mut count = 0;
        while let Ok(Ok(_)) = tokio::time::timeout(Duration::from_millis(10), receiver.recv()).await {
            count += 1;
        }
        
        // Should be rate limited (less than 150 messages)
        assert!(count < 150, "Rate limiting should prevent all messages from being sent");
        assert!(count > 0, "Some messages should still be sent");
    }

    #[tokio::test]
    async fn test_log_message_creation() {
        let data = json!({"test": "data"});
        
        let msg = LogMessage::new(LogLevel::Info, data.clone());
        assert_eq!(msg.level, LogLevel::Info);
        assert_eq!(msg.data, data);
        assert!(msg.logger.is_none());
        
        let msg_with_logger = LogMessage::with_logger(LogLevel::Error, "test-logger".to_string(), data.clone());
        assert_eq!(msg_with_logger.level, LogLevel::Error);
        assert_eq!(msg_with_logger.data, data);
        assert_eq!(msg_with_logger.logger, Some("test-logger".to_string()));
        
        let debug_msg = LogMessage::debug(data.clone());
        assert_eq!(debug_msg.level, LogLevel::Debug);
        
        let info_msg = LogMessage::info(data.clone());
        assert_eq!(info_msg.level, LogLevel::Info);
        
        let warning_msg = LogMessage::warning(data.clone());
        assert_eq!(warning_msg.level, LogLevel::Warning);
        
        let error_msg = LogMessage::error(data.clone());
        assert_eq!(error_msg.level, LogLevel::Error);
    }

    #[tokio::test]
    async fn test_concurrent_logging() {
        let logger = Arc::new(McpLogger::new());
        logger.set_level(LogLevel::Debug).unwrap();
        
        let mut receiver = logger.subscribe();
        
        // Spawn multiple tasks logging concurrently
        let mut handles = vec![];
        for i in 0..10 {
            let logger_clone = logger.clone();
            let handle = tokio::spawn(async move {
                for j in 0..5 {
                    logger_clone.info(json!({"task": i, "message": j})).unwrap();
                    sleep(Duration::from_millis(1)).await;
                }
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        // Count received notifications
        let mut count = 0;
        while let Ok(Ok(_)) = tokio::time::timeout(Duration::from_millis(100), receiver.recv()).await {
            count += 1;
        }
        
        // Should receive messages from all tasks (may be rate limited)
        assert!(count > 0, "Should receive some messages from concurrent logging");
    }
}
