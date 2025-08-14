//! MCP Notifications System
//! 
//! Provides MCP-compliant notification functionality for resource and prompt changes,
//! following the Model Context Protocol specification.

use crate::error::{Result, ProxyError};
use crate::mcp::types::McpNotification;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tracing::{debug, info};

/// MCP Notification Manager
pub struct McpNotificationManager {
    /// Broadcast channel for notifications
    notification_sender: broadcast::Sender<McpNotification>,
    /// Set of subscribed resource URIs
    resource_subscriptions: Arc<RwLock<HashSet<String>>>,
    /// Capability flags
    capabilities: NotificationCapabilities,
}

/// Notification capabilities supported by the server
#[derive(Debug, Clone)]
pub struct NotificationCapabilities {
    /// Whether resources list_changed notifications are supported
    pub resources_list_changed: bool,
    /// Whether prompts list_changed notifications are supported
    pub prompts_list_changed: bool,
    /// Whether tools list_changed notifications are supported
    pub tools_list_changed: bool,
    /// Whether resource subscriptions are supported
    pub resource_subscriptions: bool,
}

impl Default for NotificationCapabilities {
    fn default() -> Self {
        Self {
            resources_list_changed: false, // NOT IMPLEMENTED - see TODO.md
            prompts_list_changed: false,   // NOT IMPLEMENTED - see TODO.md  
            tools_list_changed: true,      // IMPLEMENTED - Full notification support
            resource_subscriptions: false, // Backend exists but MCP protocol methods not exposed
        }
    }
}

impl McpNotificationManager {
    /// Create a new notification manager
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            notification_sender: sender,
            resource_subscriptions: Arc::new(RwLock::new(HashSet::new())),
            capabilities: NotificationCapabilities::default(),
        }
    }

    /// Create a notification manager with specific capabilities
    pub fn with_capabilities(capabilities: NotificationCapabilities) -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            notification_sender: sender,
            resource_subscriptions: Arc::new(RwLock::new(HashSet::new())),
            capabilities,
        }
    }

    /// Get the notification capabilities
    pub fn capabilities(&self) -> &NotificationCapabilities {
        &self.capabilities
    }

    /// Subscribe to notifications
    pub fn subscribe(&self) -> broadcast::Receiver<McpNotification> {
        self.notification_sender.subscribe()
    }

    /// Send a notification
    fn send_notification(&self, notification: McpNotification) -> Result<()> {
        debug!("Sending MCP notification: {}", notification.method);
        
        if let Err(e) = self.notification_sender.send(notification) {
            debug!("No subscribers for notification: {}", e);
        }
        
        Ok(())
    }

    /// Notify that the resources list has changed
    pub fn notify_resources_list_changed(&self) -> Result<()> {
        if !self.capabilities.resources_list_changed {
            debug!("Resources list_changed notifications not supported");
            return Ok(());
        }

        info!("Resources list changed - sending notification");
        let notification = McpNotification::resources_list_changed();
        self.send_notification(notification)
    }

    /// Notify that the prompts list has changed
    pub fn notify_prompts_list_changed(&self) -> Result<()> {
        if !self.capabilities.prompts_list_changed {
            debug!("Prompts list_changed notifications not supported");
            return Ok(());
        }

        info!("Prompts list changed - sending notification");
        let notification = McpNotification::prompts_list_changed();
        self.send_notification(notification)
    }

    /// Notify that the tools list has changed
    pub fn notify_tools_list_changed(&self) -> Result<()> {
        if !self.capabilities.tools_list_changed {
            debug!("Tools list_changed notifications not supported");
            return Ok(());
        }

        info!("Tools list changed - sending notification");
        let notification = McpNotification::tools_list_changed();
        self.send_notification(notification)
    }

    /// Subscribe to resource updates
    pub fn subscribe_to_resource(&self, uri: String) -> Result<()> {
        if !self.capabilities.resource_subscriptions {
            return Err(ProxyError::mcp("Resource subscriptions not supported".to_string()));
        }

        let mut subscriptions = self.resource_subscriptions.write()
            .map_err(|e| ProxyError::mcp(format!("Failed to acquire write lock: {}", e)))?;
        
        if subscriptions.insert(uri.clone()) {
            info!("Subscribed to resource updates: {}", uri);
        } else {
            debug!("Already subscribed to resource: {}", uri);
        }
        
        Ok(())
    }

    /// Unsubscribe from resource updates
    pub fn unsubscribe_from_resource(&self, uri: &str) -> Result<()> {
        if !self.capabilities.resource_subscriptions {
            return Err(ProxyError::mcp("Resource subscriptions not supported".to_string()));
        }

        let mut subscriptions = self.resource_subscriptions.write()
            .map_err(|e| ProxyError::mcp(format!("Failed to acquire write lock: {}", e)))?;
        
        if subscriptions.remove(uri) {
            info!("Unsubscribed from resource updates: {}", uri);
        } else {
            debug!("Was not subscribed to resource: {}", uri);
        }
        
        Ok(())
    }

    /// Notify that a resource has been updated
    pub fn notify_resource_updated(&self, uri: String) -> Result<()> {
        if !self.capabilities.resource_subscriptions {
            debug!("Resource subscriptions not supported");
            return Ok(());
        }

        let subscriptions = self.resource_subscriptions.read()
            .map_err(|e| ProxyError::mcp(format!("Failed to acquire read lock: {}", e)))?;
        
        if subscriptions.contains(&uri) {
            info!("Resource updated - sending notification: {}", uri);
            let notification = McpNotification::resource_updated(uri);
            self.send_notification(notification)?;
        } else {
            debug!("No subscriptions for resource: {}", uri);
        }
        
        Ok(())
    }

    /// Get list of subscribed resource URIs
    pub fn get_resource_subscriptions(&self) -> Result<Vec<String>> {
        let subscriptions = self.resource_subscriptions.read()
            .map_err(|e| ProxyError::mcp(format!("Failed to acquire read lock: {}", e)))?;
        
        Ok(subscriptions.iter().cloned().collect())
    }

    /// Send a custom notification
    pub fn send_custom_notification(&self, method: String, params: Option<Value>) -> Result<()> {
        let notification = if let Some(params) = params {
            McpNotification::with_params(method, params)
        } else {
            McpNotification::new(method)
        };
        
        self.send_notification(notification)
    }

    /// Notify about tool execution events
    pub fn notify_tool_execution(&self, tool_name: &str, event: &str, data: Value) -> Result<()> {
        let notification = McpNotification::with_params(
            format!("notifications/tools/{}", event),
            serde_json::json!({
                "tool_name": tool_name,
                "event": event,
                "data": data,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })
        );
        
        self.send_notification(notification)
    }

    /// Notify about server status changes
    pub fn notify_server_status(&self, status: &str, message: Option<String>) -> Result<()> {
        let mut params = serde_json::json!({
            "status": status,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        
        if let Some(message) = message {
            params["message"] = Value::String(message);
        }
        
        let notification = McpNotification::with_params(
            "notifications/server/status".to_string(),
            params
        );
        
        self.send_notification(notification)
    }

    /// Notify about capability changes
    pub fn notify_capabilities_changed(&self, capabilities: Value) -> Result<()> {
        let notification = McpNotification::with_params(
            "notifications/server/capabilities_changed".to_string(),
            serde_json::json!({
                "capabilities": capabilities,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })
        );
        
        self.send_notification(notification)
    }

    /// Get notification statistics
    pub fn get_stats(&self) -> Result<NotificationStats> {
        let subscriptions = self.resource_subscriptions.read()
            .map_err(|e| ProxyError::mcp(format!("Failed to acquire read lock: {}", e)))?;
        
        Ok(NotificationStats {
            resource_subscriptions_count: subscriptions.len(),
            capabilities: self.capabilities.clone(),
        })
    }
}

impl Default for McpNotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Notification statistics
#[derive(Debug, Clone)]
pub struct NotificationStats {
    /// Number of active resource subscriptions
    pub resource_subscriptions_count: usize,
    /// Supported capabilities
    pub capabilities: NotificationCapabilities,
}

/// Notification event types for structured logging
#[derive(Debug, Clone)]
pub enum NotificationEvent {
    /// Resources list changed
    ResourcesListChanged,
    /// Prompts list changed
    PromptsListChanged,
    /// Resource updated
    ResourceUpdated { uri: String },
    /// Resource subscribed
    ResourceSubscribed { uri: String },
    /// Resource unsubscribed
    ResourceUnsubscribed { uri: String },
    /// Tool execution started
    ToolExecutionStarted { tool_name: String },
    /// Tool execution completed
    ToolExecutionCompleted { tool_name: String, success: bool },
    /// Server status changed
    ServerStatusChanged { status: String },
    /// Custom notification
    Custom { method: String },
}

impl NotificationEvent {
    /// Get the event type as a string
    pub fn event_type(&self) -> &'static str {
        match self {
            NotificationEvent::ResourcesListChanged => "resources_list_changed",
            NotificationEvent::PromptsListChanged => "prompts_list_changed",
            NotificationEvent::ResourceUpdated { .. } => "resource_updated",
            NotificationEvent::ResourceSubscribed { .. } => "resource_subscribed",
            NotificationEvent::ResourceUnsubscribed { .. } => "resource_unsubscribed",
            NotificationEvent::ToolExecutionStarted { .. } => "tool_execution_started",
            NotificationEvent::ToolExecutionCompleted { .. } => "tool_execution_completed",
            NotificationEvent::ServerStatusChanged { .. } => "server_status_changed",
            NotificationEvent::Custom { .. } => "custom",
        }
    }

    /// Convert to JSON value for logging
    pub fn to_json(&self) -> Value {
        match self {
            NotificationEvent::ResourcesListChanged => {
                serde_json::json!({ "event": "resources_list_changed" })
            }
            NotificationEvent::PromptsListChanged => {
                serde_json::json!({ "event": "prompts_list_changed" })
            }
            NotificationEvent::ResourceUpdated { uri } => {
                serde_json::json!({ "event": "resource_updated", "uri": uri })
            }
            NotificationEvent::ResourceSubscribed { uri } => {
                serde_json::json!({ "event": "resource_subscribed", "uri": uri })
            }
            NotificationEvent::ResourceUnsubscribed { uri } => {
                serde_json::json!({ "event": "resource_unsubscribed", "uri": uri })
            }
            NotificationEvent::ToolExecutionStarted { tool_name } => {
                serde_json::json!({ "event": "tool_execution_started", "tool_name": tool_name })
            }
            NotificationEvent::ToolExecutionCompleted { tool_name, success } => {
                serde_json::json!({ 
                    "event": "tool_execution_completed", 
                    "tool_name": tool_name, 
                    "success": success 
                })
            }
            NotificationEvent::ServerStatusChanged { status } => {
                serde_json::json!({ "event": "server_status_changed", "status": status })
            }
            NotificationEvent::Custom { method } => {
                serde_json::json!({ "event": "custom", "method": method })
            }
        }
    }
}
