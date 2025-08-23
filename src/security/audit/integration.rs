//! Audit System Integration Helpers
//! 
//! This module provides integration helpers and macros to easily integrate
//! the audit system with existing MagicTunnel components and services.

use super::{AuditEvent, AuditEventType, AuditResult, get_audit_collector};
use super::events::{AuditSeverity, EventBuilders};
use serde_json::Value;
use std::collections::HashMap;

/// Integration trait for services that want to emit audit events
pub trait AuditIntegration {
    /// Get the component name for audit events
    fn component_name(&self) -> &str;
    
    /// Emit an audit event with automatic component tagging
    async fn emit_audit_event(
        &self,
        event_type: AuditEventType,
        message: String,
        severity: AuditSeverity,
        metadata: Option<HashMap<String, Value>>,
    ) -> AuditResult<()> {
        if let Some(collector) = get_audit_collector() {
            let mut event = AuditEvent::new(event_type, self.component_name().to_string(), message)
                .with_severity(severity);
            
            if let Some(meta) = metadata {
                for (key, value) in meta {
                    event.add_metadata(&key, value);
                }
            }
            
            collector.log_event(event).await
        } else {
            Ok(()) // Audit system not initialized
        }
    }
}

/// Helper macros for common audit scenarios
pub struct AuditHelpers;

impl AuditHelpers {
    /// Log authentication event
    pub async fn log_authentication(
        component: &str,
        user_id: &str,
        success: bool,
        method: &str,
        session_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> AuditResult<()> {
        if let Some(collector) = get_audit_collector() {
            let mut event = EventBuilders::authentication(component, user_id, success, method);
            
            if let Some(session) = session_id {
                event = event.with_session(session.to_string());
            }
            
            if let Some(ip) = ip_address {
                event = event.with_network(ip.to_string(), None);
            }
            
            collector.log_event(event).await
        } else {
            Ok(())
        }
    }
    
    /// Log OAuth flow event
    pub async fn log_oauth_flow(
        component: &str,
        server_name: &str,
        flow_type: &str,
        success: bool,
        user_id: Option<&str>,
        correlation_id: Option<&str>,
    ) -> AuditResult<()> {
        if let Some(collector) = get_audit_collector() {
            let mut event = EventBuilders::oauth_flow(component, server_name, flow_type, success);
            
            if let Some(user) = user_id {
                event = event.with_user(user.to_string());
            }
            
            if let Some(corr_id) = correlation_id {
                event = event.with_correlation_id(corr_id.to_string());
            }
            
            collector.log_event(event).await
        } else {
            Ok(())
        }
    }
    
    /// Log tool execution event
    pub async fn log_tool_execution(
        component: &str,
        tool_name: &str,
        success: bool,
        user_id: Option<&str>,
        session_id: Option<&str>,
        execution_time_ms: Option<u64>,
        parameters: Option<&Value>,
    ) -> AuditResult<()> {
        if let Some(collector) = get_audit_collector() {
            let mut event = EventBuilders::tool_execution(component, tool_name, user_id, success);
            
            if let Some(session) = session_id {
                event = event.with_session(session.to_string());
            }
            
            if let Some(time) = execution_time_ms {
                event.add_metadata("execution_time_ms", serde_json::json!(time));
            }
            
            if let Some(params) = parameters {
                event.add_metadata("parameters", params.clone());
            }
            
            collector.log_event(event).await
        } else {
            Ok(())
        }
    }
    
    /// Log security violation
    pub async fn log_security_violation(
        component: &str,
        violation_type: &str,
        details: &str,
        severity: AuditSeverity,
        user_id: Option<&str>,
        blocked_action: Option<&str>,
    ) -> AuditResult<()> {
        if let Some(collector) = get_audit_collector() {
            let mut event = EventBuilders::security_violation(component, violation_type, details, severity);
            
            if let Some(user) = user_id {
                event = event.with_user(user.to_string());
            }
            
            if let Some(action) = blocked_action {
                event.add_metadata("blocked_action", serde_json::json!(action));
            }
            
            collector.log_event(event).await
        } else {
            Ok(())
        }
    }
    
    /// Log administrative action
    pub async fn log_admin_action(
        component: &str,
        admin_user: &str,
        action: &str,
        target: &str,
        session_id: Option<&str>,
        details: Option<&Value>,
    ) -> AuditResult<()> {
        if let Some(collector) = get_audit_collector() {
            let mut event = EventBuilders::admin_action(component, admin_user, action, target);
            
            if let Some(session) = session_id {
                event = event.with_session(session.to_string());
            }
            
            if let Some(detail_data) = details {
                event.add_metadata("details", detail_data.clone());
            }
            
            collector.log_event(event).await
        } else {
            Ok(())
        }
    }
    
    /// Log system performance metric
    pub async fn log_performance_metric(
        component: &str,
        metric_name: &str,
        value: f64,
        threshold: Option<f64>,
        additional_context: Option<&Value>,
    ) -> AuditResult<()> {
        if let Some(collector) = get_audit_collector() {
            let mut event = EventBuilders::system_health(component, metric_name, value, threshold);
            
            if let Some(context) = additional_context {
                event.add_metadata("context", context.clone());
            }
            
            collector.log_event(event).await
        } else {
            Ok(())
        }
    }
    
    /// Log service lifecycle event
    pub async fn log_service_lifecycle(
        component: &str,
        service_name: &str,
        lifecycle_event: &str, // "start", "stop", "restart", "health_check"
        success: bool,
        details: Option<&str>,
    ) -> AuditResult<()> {
        if let Some(collector) = get_audit_collector() {
            let event_type = match lifecycle_event {
                "start" => AuditEventType::ServiceStart,
                "stop" => AuditEventType::ServiceStop,
                _ => AuditEventType::SystemHealth,
            };
            
            let severity = if success { AuditSeverity::Info } else { AuditSeverity::Error };
            
            let message = format!("Service {} {}: {}", 
                                service_name, 
                                lifecycle_event, 
                                if success { "successful" } else { "failed" });
            
            let mut event = AuditEvent::new(event_type, component.to_string(), message)
                .with_severity(severity);
            
            event.add_metadata("service_name", serde_json::json!(service_name));
            event.add_metadata("lifecycle_event", serde_json::json!(lifecycle_event));
            event.add_metadata("success", serde_json::json!(success));
            
            if let Some(detail) = details {
                event.add_metadata("details", serde_json::json!(detail));
            }
            
            collector.log_event(event).await
        } else {
            Ok(())
        }
    }
}

/// Middleware integration helpers
pub struct AuditMiddleware;

impl AuditMiddleware {
    /// Create request context for audit trail
    pub fn create_request_context(
        request_id: String,
        user_id: Option<String>,
        session_id: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> RequestAuditContext {
        RequestAuditContext {
            request_id,
            user_id,
            session_id,
            ip_address,
            user_agent,
            start_time: std::time::Instant::now(),
        }
    }
}

/// Request-scoped audit context
#[derive(Debug, Clone)]
pub struct RequestAuditContext {
    pub request_id: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub start_time: std::time::Instant,
}

impl RequestAuditContext {
    /// Log event with this request context
    pub async fn log_event(
        &self,
        component: &str,
        event_type: AuditEventType,
        message: String,
        severity: AuditSeverity,
        additional_metadata: Option<HashMap<String, Value>>,
    ) -> AuditResult<()> {
        if let Some(collector) = get_audit_collector() {
            let mut event = AuditEvent::new(event_type, component.to_string(), message)
                .with_severity(severity)
                .with_request(self.request_id.clone());
            
            if let Some(user) = &self.user_id {
                event = event.with_user(user.clone());
            }
            
            if let Some(session) = &self.session_id {
                event = event.with_session(session.clone());
            }
            
            if let Some(ip) = &self.ip_address {
                event = event.with_network(ip.clone(), self.user_agent.clone());
            }
            
            // Add request duration
            let duration_ms = self.start_time.elapsed().as_millis() as u64;
            event.add_metadata("request_duration_ms", serde_json::json!(duration_ms));
            
            // Add any additional metadata
            if let Some(meta) = additional_metadata {
                for (key, value) in meta {
                    event.add_metadata(&key, value);
                }
            }
            
            collector.log_event(event).await
        } else {
            Ok(())
        }
    }
    
    /// Log request completion
    pub async fn log_request_completion(
        &self,
        component: &str,
        success: bool,
        response_code: Option<u16>,
        error_message: Option<&str>,
    ) -> AuditResult<()> {
        let event_type = if success { 
            AuditEventType::SystemHealth 
        } else { 
            AuditEventType::ErrorOccurred 
        };
        
        let severity = if success { AuditSeverity::Info } else { AuditSeverity::Warning };
        
        let message = format!("Request {} completed: {}", 
                            self.request_id, 
                            if success { "success" } else { "failed" });
        
        let mut metadata = HashMap::new();
        metadata.insert("success".to_string(), serde_json::json!(success));
        
        if let Some(code) = response_code {
            metadata.insert("response_code".to_string(), serde_json::json!(code));
        }
        
        if let Some(error) = error_message {
            metadata.insert("error_message".to_string(), serde_json::json!(error));
        }
        
        self.log_event(component, event_type, message, severity, Some(metadata)).await
    }
}

/// Service integration helper for easy implementation
#[macro_export]
macro_rules! implement_audit_integration {
    ($service_type:ty, $component_name:expr) => {
        impl $crate::security::audit::integration::AuditIntegration for $service_type {
            fn component_name(&self) -> &str {
                $component_name
            }
        }
    };
}

/// Convenience macro for logging audit events
#[macro_export]
macro_rules! audit_event {
    ($component:expr, $event_type:expr, $message:expr) => {
        $crate::audit_log!($event_type, $component, $message)
    };
    
    ($component:expr, $event_type:expr, $message:expr, $severity:expr) => {
        if let Some(collector) = $crate::security::audit::get_audit_collector() {
            let event = $crate::security::audit::AuditEvent::new(
                $event_type,
                $component.to_string(),
                $message.to_string(),
            ).with_severity($severity);
            let _ = collector.log_event(event).await;
        }
    };
    
    ($component:expr, $event_type:expr, $message:expr, $severity:expr, $($key:expr => $value:expr),*) => {
        if let Some(collector) = $crate::security::audit::get_audit_collector() {
            let mut event = $crate::security::audit::AuditEvent::new(
                $event_type,
                $component.to_string(),
                $message.to_string(),
            ).with_severity($severity);
            $(
                event.add_metadata($key, serde_json::json!($value));
            )*
            let _ = collector.log_event(event).await;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::audit::{AuditConfig, initialize_audit_system};
    
    struct TestService;
    
    implement_audit_integration!(TestService, "test_service");
    
    #[tokio::test]
    async fn test_audit_integration_trait() {
        // Initialize audit system for test
        let config = AuditConfig::default();
        initialize_audit_system(config).await.unwrap();
        
        let service = TestService;
        assert_eq!(service.component_name(), "test_service");
        
        let result = service.emit_audit_event(
            AuditEventType::ToolExecution,
            "Test event".to_string(),
            AuditSeverity::Info,
            None,
        ).await;
        
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_request_audit_context() {
        let context = AuditMiddleware::create_request_context(
            "req_123".to_string(),
            Some("user_456".to_string()),
            Some("session_789".to_string()),
            Some("192.168.1.1".to_string()),
            Some("test_agent".to_string()),
        );
        
        assert_eq!(context.request_id, "req_123");
        assert_eq!(context.user_id, Some("user_456".to_string()));
    }
}