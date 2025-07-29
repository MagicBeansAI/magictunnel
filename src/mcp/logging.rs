//! MCP Logging System
//! 
//! Provides MCP-compliant logging functionality following the Model Context Protocol
//! specification for structured log messages and notifications.

use crate::error::{Result, ProxyError};
use crate::mcp::types::{LogLevel, LogMessage, McpNotification};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tracing::{debug, info, warn, error};

/// MCP Logger that manages log levels and emits notifications
pub struct McpLogger {
    /// Current minimum log level
    min_level: Arc<RwLock<LogLevel>>,
    /// Broadcast channel for log notifications
    notification_sender: broadcast::Sender<McpNotification>,
    /// Logger name for this instance
    logger_name: Option<String>,
    /// Rate limiting state
    rate_limiter: Arc<RwLock<RateLimiter>>,
}

/// Simple rate limiter for log messages
#[derive(Debug)]
struct RateLimiter {
    /// Message counts per logger per minute
    message_counts: HashMap<String, u32>,
    /// Last reset time
    last_reset: std::time::Instant,
    /// Maximum messages per minute per logger
    max_messages_per_minute: u32,
}

impl RateLimiter {
    fn new(max_messages_per_minute: u32) -> Self {
        Self {
            message_counts: HashMap::new(),
            last_reset: std::time::Instant::now(),
            max_messages_per_minute,
        }
    }

    fn should_allow(&mut self, logger: &str) -> bool {
        // Reset counts every minute
        if self.last_reset.elapsed().as_secs() >= 60 {
            self.message_counts.clear();
            self.last_reset = std::time::Instant::now();
        }

        let count = self.message_counts.entry(logger.to_string()).or_insert(0);
        if *count >= self.max_messages_per_minute {
            false
        } else {
            *count += 1;
            true
        }
    }
}

impl McpLogger {
    /// Create a new MCP logger
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            min_level: Arc::new(RwLock::new(LogLevel::Info)),
            notification_sender: sender,
            logger_name: None,
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new(100))), // 100 messages per minute
        }
    }

    /// Create a new MCP logger with a specific name
    pub fn with_name(name: String) -> Self {
        let mut logger = Self::new();
        logger.logger_name = Some(name);
        logger
    }

    /// Set the minimum log level
    pub fn set_level(&self, level: LogLevel) -> Result<()> {
        let mut min_level = self.min_level.write()
            .map_err(|e| ProxyError::mcp(format!("Failed to acquire write lock: {}", e)))?;
        *min_level = level;
        
        info!("MCP log level set to {:?}", level);
        Ok(())
    }

    /// Get the current minimum log level
    pub fn get_level(&self) -> Result<LogLevel> {
        let min_level = self.min_level.read()
            .map_err(|e| ProxyError::mcp(format!("Failed to acquire read lock: {}", e)))?;
        Ok(*min_level)
    }

    /// Subscribe to log notifications
    pub fn subscribe(&self) -> broadcast::Receiver<McpNotification> {
        self.notification_sender.subscribe()
    }

    /// Log a message if it meets the minimum level requirement
    pub fn log(&self, level: LogLevel, data: Value) -> Result<()> {
        let min_level = self.get_level()?;
        
        if !level.should_log(min_level) {
            return Ok(());
        }

        // Check rate limiting
        let logger_key = self.logger_name.as_deref().unwrap_or("default");
        {
            let mut rate_limiter = self.rate_limiter.write()
                .map_err(|e| ProxyError::mcp(format!("Failed to acquire rate limiter lock: {}", e)))?;
            
            if !rate_limiter.should_allow(logger_key) {
                debug!("Rate limiting log message from logger: {}", logger_key);
                return Ok(());
            }
        }

        // Create log message
        let log_message = if let Some(ref logger_name) = self.logger_name {
            LogMessage::with_logger(level, logger_name.clone(), data.clone())
        } else {
            LogMessage::new(level, data.clone())
        };

        // Send notification
        let notification = McpNotification::log_message(log_message);
        if let Err(e) = self.notification_sender.send(notification) {
            debug!("No subscribers for log notification: {}", e);
        }

        // Also log to tracing for local debugging
        match level {
            LogLevel::Debug => debug!("MCP Log: {}", data),
            LogLevel::Info => info!("MCP Log: {}", data),
            LogLevel::Notice => info!("MCP Notice: {}", data),
            LogLevel::Warning => warn!("MCP Warning: {}", data),
            LogLevel::Error => error!("MCP Error: {}", data),
            LogLevel::Critical => error!("MCP Critical: {}", data),
            LogLevel::Alert => error!("MCP Alert: {}", data),
            LogLevel::Emergency => error!("MCP Emergency: {}", data),
        }

        Ok(())
    }

    /// Log a debug message
    pub fn debug<T: Into<Value>>(&self, data: T) -> Result<()> {
        self.log(LogLevel::Debug, data.into())
    }

    /// Log an info message
    pub fn info<T: Into<Value>>(&self, data: T) -> Result<()> {
        self.log(LogLevel::Info, data.into())
    }

    /// Log a notice message
    pub fn notice<T: Into<Value>>(&self, data: T) -> Result<()> {
        self.log(LogLevel::Notice, data.into())
    }

    /// Log a warning message
    pub fn warning<T: Into<Value>>(&self, data: T) -> Result<()> {
        self.log(LogLevel::Warning, data.into())
    }

    /// Log an error message
    pub fn error<T: Into<Value>>(&self, data: T) -> Result<()> {
        self.log(LogLevel::Error, data.into())
    }

    /// Log a critical message
    pub fn critical<T: Into<Value>>(&self, data: T) -> Result<()> {
        self.log(LogLevel::Critical, data.into())
    }

    /// Log an alert message
    pub fn alert<T: Into<Value>>(&self, data: T) -> Result<()> {
        self.log(LogLevel::Alert, data.into())
    }

    /// Log an emergency message
    pub fn emergency<T: Into<Value>>(&self, data: T) -> Result<()> {
        self.log(LogLevel::Emergency, data.into())
    }

    /// Log tool execution start
    pub fn log_tool_start(&self, tool_name: &str, arguments: &Value) -> Result<()> {
        self.info(json!({
            "event": "tool_execution_start",
            "tool_name": tool_name,
            "arguments": arguments,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// Log tool execution success
    pub fn log_tool_success(&self, tool_name: &str, duration_ms: u64, result_size: usize) -> Result<()> {
        self.info(json!({
            "event": "tool_execution_success",
            "tool_name": tool_name,
            "duration_ms": duration_ms,
            "result_size_bytes": result_size,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// Log tool execution error
    pub fn log_tool_error(&self, tool_name: &str, error: &str, duration_ms: u64) -> Result<()> {
        self.error(json!({
            "event": "tool_execution_error",
            "tool_name": tool_name,
            "error": error,
            "duration_ms": duration_ms,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// Log resource access
    pub fn log_resource_access(&self, uri: &str, operation: &str) -> Result<()> {
        self.info(json!({
            "event": "resource_access",
            "uri": uri,
            "operation": operation,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// Log prompt template usage
    pub fn log_prompt_usage(&self, template_name: &str, arguments: &Value) -> Result<()> {
        self.info(json!({
            "event": "prompt_template_usage",
            "template_name": template_name,
            "arguments": arguments,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

impl Default for McpLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Global MCP logger manager
pub struct McpLoggerManager {
    /// Default logger
    default_logger: Arc<McpLogger>,
    /// Named loggers
    loggers: Arc<RwLock<HashMap<String, Arc<McpLogger>>>>,
}

impl McpLoggerManager {
    /// Create a new logger manager
    pub fn new() -> Self {
        Self {
            default_logger: Arc::new(McpLogger::new()),
            loggers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the default logger
    pub fn default_logger(&self) -> Arc<McpLogger> {
        self.default_logger.clone()
    }

    /// Get or create a named logger
    pub fn get_logger(&self, name: &str) -> Result<Arc<McpLogger>> {
        let loggers = self.loggers.read()
            .map_err(|e| ProxyError::mcp(format!("Failed to acquire read lock: {}", e)))?;
        
        if let Some(logger) = loggers.get(name) {
            Ok(logger.clone())
        } else {
            drop(loggers);
            
            let mut loggers = self.loggers.write()
                .map_err(|e| ProxyError::mcp(format!("Failed to acquire write lock: {}", e)))?;
            
            // Double-check in case another thread created it
            if let Some(logger) = loggers.get(name) {
                Ok(logger.clone())
            } else {
                let logger = Arc::new(McpLogger::with_name(name.to_string()));
                loggers.insert(name.to_string(), logger.clone());
                Ok(logger)
            }
        }
    }

    /// Set log level for all loggers
    pub fn set_global_level(&self, level: LogLevel) -> Result<()> {
        self.default_logger.set_level(level)?;
        
        let loggers = self.loggers.read()
            .map_err(|e| ProxyError::mcp(format!("Failed to acquire read lock: {}", e)))?;
        
        for logger in loggers.values() {
            logger.set_level(level)?;
        }
        
        Ok(())
    }
}

impl Default for McpLoggerManager {
    fn default() -> Self {
        Self::new()
    }
}
