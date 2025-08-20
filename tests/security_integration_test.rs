//! Integration tests for MagicTunnel security features
//!
//! Tests the complete security pipeline including allowlisting, sanitization,
//! RBAC, policies, and audit logging.

use std::collections::HashMap;
use chrono::Utc;
use serde_json::json;

use magictunnel::security::{
    SecurityConfig, SecurityMiddleware, SecurityContext, SecurityUser, SecurityRequest, SecurityTool,
    AllowlistConfig, AllowlistAction, SanitizationConfig, RbacConfig,
    AllowlistRule
};

#[tokio::test]
async fn test_security_integration_basic() {
    // Create a basic security configuration
    let mut security_config = SecurityConfig::default();
    security_config.enabled = true;
    
    // Enable basic allowlisting
    security_config.allowlist = Some(AllowlistConfig {
        enabled: true,
        default_action: AllowlistAction::Allow,
        emergency_lockdown: false,
        tools: HashMap::new(),
        tool_patterns: vec![],
        capabilities: HashMap::new(),
        capability_patterns: vec![],
        global_patterns: vec![],
        mt_level_rules: HashMap::new(),
        data_file: "./security/allowlist-data.yaml".to_string(),
    });
    
    // Initialize security middleware
    let security_middleware = SecurityMiddleware::new(security_config).await.unwrap();
    
    // Create test security context
    let user = Some(SecurityUser {
        id: Some("test-user".to_string()),
        roles: vec!["user".to_string()],
        permissions: vec!["read".to_string()],
        api_key_name: None,
        auth_method: "test".to_string(),
    });
    
    let request = SecurityRequest {
        id: "test-123".to_string(),
        method: "POST".to_string(),
        path: "/mcp/call/test_tool".to_string(),
        client_ip: Some("127.0.0.1".to_string()),
        user_agent: Some("test-agent".to_string()),
        headers: HashMap::new(),
        body: Some(r#"{"param": "value"}"#.to_string()),
        timestamp: Utc::now(),
    };
    
    let tool = Some(SecurityTool {
        name: "test_tool".to_string(),
        parameters: {
            let mut params = HashMap::new();
            params.insert("param".to_string(), json!("value"));
            params
        },
        source: Some("test_capability".to_string()),
    });
    
    let context = SecurityContext {
        user,
        request,
        tool,
        resource: None,
        metadata: HashMap::new(),
    };
    
    // Evaluate security
    let result = security_middleware.evaluate_security(&context).await;
    
    // Should be allowed with basic configuration
    assert!(result.allowed);
    assert!(!result.blocked);
    assert!(!result.requires_approval);
}

#[tokio::test]
async fn test_security_integration_blocked() {
    // Create a strict security configuration
    let mut security_config = SecurityConfig::default();
    security_config.enabled = true;
    
    // Enable strict allowlisting that blocks by default
    let mut tool_rules = HashMap::new();
    tool_rules.insert("allowed_tool".to_string(), AllowlistRule {
        action: AllowlistAction::Allow,
        reason: Some("Explicitly allowed tool".to_string()),
        pattern: None,
        name: Some("allowed_tool".to_string()),
        enabled: true,
    });
    
    security_config.allowlist = Some(AllowlistConfig {
        enabled: true,
        default_action: AllowlistAction::Deny,
        emergency_lockdown: false,
        tools: tool_rules,
        tool_patterns: vec![],
        capabilities: HashMap::new(),
        capability_patterns: vec![],
        global_patterns: vec![],
        mt_level_rules: HashMap::new(),
        data_file: "./security/allowlist-data.yaml".to_string(),
    });
    
    // Initialize security middleware
    let security_middleware = SecurityMiddleware::new(security_config).await.unwrap();
    
    // Create test security context for blocked tool
    let user = Some(SecurityUser {
        id: Some("test-user".to_string()),
        roles: vec!["user".to_string()],
        permissions: vec!["read".to_string()],
        api_key_name: None,
        auth_method: "test".to_string(),
    });
    
    let request = SecurityRequest {
        id: "test-123".to_string(),
        method: "POST".to_string(),
        path: "/mcp/call/blocked_tool".to_string(),
        client_ip: Some("127.0.0.1".to_string()),
        user_agent: Some("test-agent".to_string()),
        headers: HashMap::new(),
        body: Some(r#"{"param": "value"}"#.to_string()),
        timestamp: Utc::now(),
    };
    
    let tool = Some(SecurityTool {
        name: "blocked_tool".to_string(),
        parameters: {
            let mut params = HashMap::new();
            params.insert("param".to_string(), json!("value"));
            params
        },
        source: Some("test_capability".to_string()),
    });
    
    let context = SecurityContext {
        user,
        request,
        tool,
        resource: None,
        metadata: HashMap::new(),
    };
    
    // Evaluate security
    let result = security_middleware.evaluate_security(&context).await;
    
    // Should be blocked
    assert!(!result.allowed);
    assert!(result.blocked);
    assert!(!result.requires_approval);
    assert!(result.reason.contains("allowlist") || result.reason.contains("default"));
}

#[tokio::test]
async fn test_security_integration_rbac() {
    // Create security configuration with RBAC
    let mut security_config = SecurityConfig::default();
    security_config.enabled = true;
    
    // Enable RBAC
    security_config.rbac = Some(RbacConfig {
        enabled: true,
        ..Default::default()
    });
    
    // Initialize security middleware
    let security_middleware = SecurityMiddleware::new(security_config).await.unwrap();
    
    // Create test security context with user role
    let user = Some(SecurityUser {
        id: Some("test-user".to_string()),
        roles: vec!["user".to_string()],
        permissions: vec!["read".to_string()],
        api_key_name: None,
        auth_method: "test".to_string(),
    });
    
    let request = SecurityRequest {
        id: "test-123".to_string(),
        method: "GET".to_string(), // GET requires 'read' permission
        path: "/mcp/tools".to_string(),
        client_ip: Some("127.0.0.1".to_string()),
        user_agent: Some("test-agent".to_string()),
        headers: HashMap::new(),
        body: None,
        timestamp: Utc::now(),
    };
    
    let context = SecurityContext {
        user,
        request,
        tool: None,
        resource: None,
        metadata: HashMap::new(),
    };
    
    // Evaluate security
    let result = security_middleware.evaluate_security(&context).await;
    
    // Should be allowed for user with read permission
    assert!(result.allowed);
    assert!(!result.blocked);
}

#[tokio::test]
async fn test_security_integration_sanitization() {
    // Create security configuration with sanitization
    let mut security_config = SecurityConfig::default();
    security_config.enabled = true;
    
    // Enable sanitization
    security_config.sanitization = Some(SanitizationConfig {
        enabled: true,
        ..Default::default()
    });
    
    // Initialize security middleware
    let security_middleware = SecurityMiddleware::new(security_config).await.unwrap();
    
    // Create test security context with potentially sensitive data
    let user = Some(SecurityUser {
        id: Some("test-user".to_string()),
        roles: vec!["user".to_string()],
        permissions: vec!["read".to_string(), "write".to_string()],
        api_key_name: None,
        auth_method: "test".to_string(),
    });
    
    let request = SecurityRequest {
        id: "test-123".to_string(),
        method: "POST".to_string(),
        path: "/mcp/call/test_tool".to_string(),
        client_ip: Some("127.0.0.1".to_string()),
        user_agent: Some("test-agent".to_string()),
        headers: HashMap::new(),
        body: Some(r#"{"api_key": "sk-1234567890abcdef1234567890abcdef", "data": "normal data"}"#.to_string()),
        timestamp: Utc::now(),
    };
    
    let tool = Some(SecurityTool {
        name: "test_tool".to_string(),
        parameters: {
            let mut params = HashMap::new();
            params.insert("api_key".to_string(), json!("sk-1234567890abcdef1234567890abcdef"));
            params.insert("data".to_string(), json!("normal data"));
            params
        },
        source: Some("test_capability".to_string()),
    });
    
    let context = SecurityContext {
        user,
        request,
        tool,
        resource: None,
        metadata: HashMap::new(),
    };
    
    // Evaluate security
    let result = security_middleware.evaluate_security(&context).await;
    
    // Should be allowed since sanitization is enabled but not configured with specific blocking patterns
    // The current implementation enables sanitization service but doesn't have specific patterns to block API keys
    // This test verifies that sanitization service can be initialized and runs successfully
    assert!(result.allowed);
    assert!(!result.blocked);
    assert!(!result.requires_approval);
}

#[tokio::test]
async fn test_security_middleware_disabled() {
    // Create disabled security configuration
    let security_config = SecurityConfig {
        enabled: false,
        ..Default::default()
    };
    
    // Initialize security middleware
    let security_middleware = SecurityMiddleware::new(security_config).await.unwrap();
    
    // Create minimal test context
    let context = SecurityContext {
        user: None,
        request: SecurityRequest {
            id: "test-123".to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            client_ip: None,
            user_agent: None,
            headers: HashMap::new(),
            body: None,
            timestamp: Utc::now(),
        },
        tool: None,
        resource: None,
        metadata: HashMap::new(),
    };
    
    // Evaluate security
    let result = security_middleware.evaluate_security(&context).await;
    
    // Should always be allowed when disabled
    assert!(result.allowed);
    assert!(!result.blocked);
    assert!(!result.requires_approval);
    assert!(result.reason.contains("disabled"));
}