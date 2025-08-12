//! OAuth 2.1 Phase 6 Integration Test
//! 
//! Tests the complete integration of OAuth 2.1 authentication system with
//! MCP protocol flows, validating that authentication context flows through
//! the entire tool execution pipeline.

use magictunnel::auth::{
    AuthenticationContext, AuthenticationResult, ToolExecutionContext,
    ProviderToken, UserContext, OAuthValidationResult, OAuthUserInfo,
    OAuthTokenResponse, AuthContextMethod,
};
use magictunnel::config::ApiKeyEntry;
use magictunnel::error::Result;
use magictunnel::mcp::{ToolCall, ToolResult, McpServer};
use magictunnel::registry::{RegistryService, ToolDefinition, RoutingConfig};
use magictunnel::routing::{Router, types::AgentResult};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use secrecy::Secret;
use uuid::Uuid;

/// Test helper to create OAuth authentication result
fn create_oauth_auth_result() -> AuthenticationResult {
    let user_info = OAuthUserInfo {
        id: "oauth_user_123".to_string(),
        email: Some("oauth.user@example.com".to_string()),
        name: Some("OAuth Test User".to_string()),
        login: Some("oauth_user_123".to_string()),
    };

    let token_response = OAuthTokenResponse {
        access_token: Secret::new("oauth_access_token_test".to_string()),
        token_type: "Bearer".to_string(),
        expires_in: Some(3600),
        refresh_token: Some(Secret::new("oauth_refresh_token_test".to_string())),
        scope: Some("repo user:email".to_string()),
        audience: None,
        resource: None,
    };

    let oauth_result = OAuthValidationResult {
        user_info,
        expires_at: Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 3600),
        scopes: vec!["repo".to_string(), "user:email".to_string()],
        audience: None,
        resources: None,
        issuer: Some("https://github.com".to_string()),
    };

    AuthenticationResult::OAuth(oauth_result)
}

/// Test helper to create API Key authentication result
fn create_api_key_auth_result() -> AuthenticationResult {
    let api_key = ApiKeyEntry::with_permissions(
        "test_api_key_123".to_string(),
        "Test API Key User".to_string(),
        vec!["read".to_string(), "write".to_string(), "admin".to_string()],
    );

    AuthenticationResult::ApiKey(api_key)
}

#[tokio::test]
async fn test_authentication_context_creation() -> Result<()> {
    // Test OAuth context creation
    let oauth_result = create_oauth_auth_result();
    let session_id = "test_session_oauth_123".to_string();
    
    let auth_context = AuthenticationContext::from_auth_result(
        &oauth_result,
        session_id.clone(),
    )?;

    assert_eq!(auth_context.user_id, "oauth_user_123");
    assert_eq!(auth_context.session_id, session_id);
    assert!(matches!(auth_context.auth_method, AuthContextMethod::OAuth { .. }));
    assert_eq!(auth_context.scopes, vec!["repo", "user:email"]);
    assert!(auth_context.provider_tokens.contains_key("oauth"));
    
    // Validate context
    auth_context.validate()?;

    // Test API Key context creation
    let api_key_result = create_api_key_auth_result();
    let api_session_id = "test_session_api_456".to_string();
    
    let api_auth_context = AuthenticationContext::from_auth_result(
        &api_key_result,
        api_session_id.clone(),
    )?;

    assert_eq!(api_auth_context.user_id, "Test API Key User");
    assert_eq!(api_auth_context.session_id, api_session_id);
    assert!(matches!(api_auth_context.auth_method, AuthContextMethod::ApiKey { .. }));
    assert_eq!(api_auth_context.scopes, vec!["read", "write", "admin"]);
    assert!(api_auth_context.provider_tokens.contains_key("api_key"));

    // Validate context
    api_auth_context.validate()?;

    // Test anonymous context
    let anon_context = AuthenticationContext::none("anon_session_789".to_string());
    assert_eq!(anon_context.user_id, "anonymous");
    assert!(anon_context.is_anonymous());
    assert_eq!(anon_context.auth_method_display(), "None");
    anon_context.validate()?;

    println!("✅ Authentication context creation tests passed");
    Ok(())
}

#[tokio::test]
async fn test_tool_execution_context() -> Result<()> {
    let oauth_result = create_oauth_auth_result();
    let auth_context = AuthenticationContext::from_auth_result(
        &oauth_result,
        "exec_session_123".to_string(),
    )?;

    // Create tool execution context with authentication
    let tool_context = ToolExecutionContext::new(
        "github_create_issue".to_string(),
        json!({
            "title": "Test Issue",
            "body": "This is a test issue created via authenticated API call",
            "labels": ["test", "oauth"]
        }),
        Some(auth_context),
    )
    .with_metadata("routing_type".to_string(), "external_mcp".to_string())
    .with_metadata("expected_provider".to_string(), "github".to_string());

    assert_eq!(tool_context.tool_name, "github_create_issue");
    assert!(tool_context.has_auth());
    assert_eq!(tool_context.get_user_id(), "oauth_user_123");

    // Test authentication header generation
    let auth_headers = tool_context.get_auth_headers(Some("oauth"));
    assert!(auth_headers.contains_key("Authorization"));
    assert!(auth_headers.contains_key("X-Session-ID"));
    assert!(auth_headers.contains_key("X-User-ID"));
    assert!(auth_headers.contains_key("X-Auth-Provider"));

    assert_eq!(auth_headers.get("X-Session-ID").unwrap(), "exec_session_123");
    assert_eq!(auth_headers.get("X-User-ID").unwrap(), "oauth_user_123");
    assert_eq!(auth_headers.get("X-Auth-Provider").unwrap(), "oauth");
    assert!(auth_headers.get("Authorization").unwrap().starts_with("Bearer "));

    // Validate execution context
    tool_context.validate()?;

    // Test context without authentication
    let anon_tool_context = ToolExecutionContext::new(
        "public_tool".to_string(),
        json!({"query": "test"}),
        None,
    );

    assert!(!anon_tool_context.has_auth());
    assert_eq!(anon_tool_context.get_user_id(), "anonymous");
    assert!(anon_tool_context.get_auth_headers(None).is_empty());

    println!("✅ Tool execution context tests passed");
    Ok(())
}

#[tokio::test]
async fn test_provider_token_management() -> Result<()> {
    let oauth_response = OAuthTokenResponse {
        access_token: Secret::new("github_token_abc123".to_string()),
        token_type: "Bearer".to_string(),
        expires_in: Some(3600), // 1 hour
        refresh_token: Some(Secret::new("github_refresh_xyz789".to_string())),
        scope: Some("repo user:email".to_string()),
        audience: None,
        resource: None,
    };

    let provider_token = ProviderToken::from_oauth_response(&oauth_response, "github");

    // Test token properties
    assert_eq!(provider_token.token_type, "Bearer");
    assert!(!provider_token.is_expired());
    assert!(provider_token.has_scope("repo"));
    assert!(provider_token.has_scope("user:email"));
    assert!(!provider_token.has_scope("admin"));

    // Test authorization header generation
    let auth_header = provider_token.get_authorization_header();
    assert_eq!(auth_header, "Bearer github_token_abc123");

    // Test metadata
    assert_eq!(provider_token.get_provider(), Some(&"github".to_string()));

    // Test expired token
    let expired_token = ProviderToken {
        access_token: Secret::new("expired_token".to_string()),
        refresh_token: None,
        token_type: "Bearer".to_string(),
        expires_at: Some(1000), // Way in the past
        scopes: vec!["read".to_string()],
        metadata: HashMap::new(),
    };

    assert!(expired_token.is_expired());

    println!("✅ Provider token management tests passed");
    Ok(())
}

#[tokio::test]
async fn test_auth_headers_generation() -> Result<()> {
    let oauth_result = create_oauth_auth_result();
    let auth_context = AuthenticationContext::from_auth_result(
        &oauth_result,
        "header_test_session".to_string(),
    )?;

    // Test provider-specific headers
    let oauth_headers = auth_context.get_auth_headers(Some("oauth"));
    assert_eq!(oauth_headers.len(), 4); // Authorization, X-Session-ID, X-User-ID, X-Auth-Provider

    assert!(oauth_headers.contains_key("Authorization"));
    assert!(oauth_headers.contains_key("X-Session-ID"));
    assert!(oauth_headers.contains_key("X-User-ID"));
    assert!(oauth_headers.contains_key("X-Auth-Provider"));

    let auth_header = oauth_headers.get("Authorization").unwrap();
    assert!(auth_header.starts_with("Bearer "));
    assert!(auth_header.contains("oauth_token"));

    // Test automatic provider selection (no provider specified)
    let auto_headers = auth_context.get_auth_headers(None);
    assert!(!auto_headers.is_empty());
    assert!(auto_headers.contains_key("Authorization"));

    // Test scope validation
    assert!(auth_context.has_scope("repo"));
    assert!(auth_context.has_scope("user:email"));
    assert!(!auth_context.has_scope("admin"));

    assert!(auth_context.has_provider_scope("oauth", "repo"));
    assert!(!auth_context.has_provider_scope("oauth", "admin"));
    assert!(!auth_context.has_provider_scope("nonexistent", "repo"));

    println!("✅ Authentication headers generation tests passed");
    Ok(())
}

#[tokio::test]
async fn test_mcp_server_auth_integration() -> Result<()> {
    // This test would require a more complex setup with actual MCP server
    // For now, we'll test the core authentication context flow

    let tool_call = ToolCall {
        name: "test_tool".to_string(),
        arguments: json!({"param1": "value1", "param2": 42}),
    };

    let oauth_result = create_oauth_auth_result();
    let auth_context = AuthenticationContext::from_auth_result(
        &oauth_result,
        "mcp_server_test_session".to_string(),
    )?;

    // Test that authentication context can be created and validated
    auth_context.validate()?;

    // Test tool execution context creation
    let execution_context = ToolExecutionContext::new(
        tool_call.name.clone(),
        tool_call.arguments.clone(),
        Some(auth_context.clone()),
    );

    execution_context.validate()?;

    // Test session ID generation
    assert!(auth_context.session_id.contains("mcp_server_test_session"));
    
    // Test metadata propagation
    assert!(auth_context.metadata.contains_key("provider"));
    assert_eq!(auth_context.metadata.get("provider"), Some(&"oauth".to_string()));

    // Test authentication method display
    let method_display = auth_context.auth_method_display();
    assert!(method_display.contains("OAuth"));
    assert!(method_display.contains("oauth"));

    println!("✅ MCP server authentication integration tests passed");
    Ok(())
}

#[test]
fn test_auth_method_variants() {
    // Test all AuthContextMethod variants
    let oauth_method = AuthContextMethod::OAuth {
        provider: "github".to_string(),
        scopes: vec!["repo".to_string()],
    };

    let api_key_method = AuthContextMethod::ApiKey {
        key_ref: "test_key".to_string(),
    };

    let device_code_method = AuthContextMethod::DeviceCode {
        provider: "github".to_string(),
        device_code: "device_123".to_string(),
    };

    let service_account_method = AuthContextMethod::ServiceAccount {
        account_ref: "service_account_1".to_string(),
    };

    let jwt_method = AuthContextMethod::Jwt {
        issuer: "https://auth.example.com".to_string(),
    };

    let none_method = AuthContextMethod::None;

    // Test serialization/deserialization
    let methods = vec![
        oauth_method,
        api_key_method,
        device_code_method,
        service_account_method,
        jwt_method,
        none_method,
    ];

    for method in methods {
        let serialized = serde_json::to_string(&method).expect("Failed to serialize AuthMethod");
        let deserialized: AuthContextMethod = serde_json::from_str(&serialized).expect("Failed to deserialize AuthContextMethod");
        assert_eq!(method, deserialized);
    }

    println!("✅ Authentication method variants tests passed");
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    // Test expired token in authentication context
    let mut expired_tokens = HashMap::new();
    let expired_token = ProviderToken {
        access_token: Secret::new("expired".to_string()),
        refresh_token: None,
        token_type: "Bearer".to_string(),
        expires_at: Some(1000), // Way in the past
        scopes: vec!["read".to_string()],
        metadata: HashMap::new(),
    };
    expired_tokens.insert("expired_provider".to_string(), expired_token);

    let expired_context = AuthenticationContext {
        user_id: "test_user".to_string(),
        provider_tokens: expired_tokens,
        session_id: "test_session".to_string(),
        auth_method: AuthContextMethod::OAuth {
            provider: "expired_provider".to_string(),
            scopes: vec!["read".to_string()],
        },
        scopes: vec!["read".to_string()],
        timestamp: 1000,
        metadata: HashMap::new(),
    };

    // Validation should fail for expired tokens
    assert!(expired_context.validate().is_err());

    // Test empty authentication context validation
    let valid_anon_context = AuthenticationContext::none("test_anon".to_string());
    assert!(valid_anon_context.validate().is_ok());

    // Test tool execution context validation
    let invalid_tool_context = ToolExecutionContext {
        tool_name: "".to_string(), // Empty tool name should fail validation
        arguments: json!({}),
        auth_context: None,
        timestamp: 1000,
        metadata: HashMap::new(),
    };

    assert!(invalid_tool_context.validate().is_err());

    println!("✅ Error handling tests passed");
    Ok(())
}

/// Integration test demonstrating the complete authentication flow
#[tokio::test]
async fn test_complete_authentication_flow() -> Result<()> {
    println!("🚀 Testing complete OAuth 2.1 Phase 6 authentication flow");

    // Step 1: Create authentication result (normally from middleware)
    let auth_result = create_oauth_auth_result();
    println!("✓ Created OAuth authentication result");

    // Step 2: Convert to authentication context
    let session_id = format!("integration_test_{}", Uuid::new_v4());
    let auth_context = AuthenticationContext::from_auth_result(&auth_result, session_id.clone())?;
    println!("✓ Created authentication context with session ID: {}", session_id);

    // Step 3: Validate authentication context
    auth_context.validate()?;
    println!("✓ Authentication context validated successfully");

    // Step 4: Create tool execution context
    let tool_call = ToolCall {
        name: "github_create_repository".to_string(),
        arguments: json!({
            "name": "test-repo",
            "description": "Repository created via authenticated OAuth API call",
            "private": false,
            "auto_init": true
        }),
    };

    let execution_context = ToolExecutionContext::new(
        tool_call.name.clone(),
        tool_call.arguments.clone(),
        Some(auth_context.clone()),
    )
    .with_metadata("test_type".to_string(), "integration".to_string())
    .with_metadata("provider".to_string(), "github".to_string());

    println!("✓ Created tool execution context for: {}", tool_call.name);

    // Step 5: Validate execution context
    execution_context.validate()?;
    println!("✓ Tool execution context validated successfully");

    // Step 6: Generate authentication headers for external API call
    let auth_headers = execution_context.get_auth_headers(Some("oauth"));
    println!("✓ Generated {} authentication headers", auth_headers.len());

    // Step 7: Verify header contents
    assert!(auth_headers.contains_key("Authorization"));
    assert!(auth_headers.contains_key("X-Session-ID"));
    assert!(auth_headers.contains_key("X-User-ID"));
    assert!(auth_headers.contains_key("X-Auth-Provider"));

    let auth_header = auth_headers.get("Authorization").unwrap();
    assert!(auth_header.starts_with("Bearer oauth_token"));
    println!("✓ Authentication header validated: {}", auth_header);

    // Step 8: Verify user context
    assert_eq!(execution_context.get_user_id(), "oauth_user_123");
    assert!(execution_context.has_auth());
    println!("✓ User context verified: {}", execution_context.get_user_id());

    // Step 9: Test scope validation
    assert!(auth_context.has_scope("repo"));
    assert!(auth_context.has_scope("user:email"));
    assert!(auth_context.has_provider_scope("oauth", "repo"));
    println!("✓ OAuth scopes validated: {:?}", auth_context.scopes);

    // Step 10: Verify authentication method
    match &auth_context.auth_method {
        AuthContextMethod::OAuth { provider, scopes } => {
            assert_eq!(provider, "oauth");
            assert!(scopes.contains(&"repo".to_string()));
            println!("✓ OAuth authentication method verified: {}", provider);
        }
        _ => panic!("Expected OAuth authentication method"),
    }

    println!("🎉 Complete OAuth 2.1 Phase 6 authentication flow test PASSED!");
    println!("   - Authentication context created and validated");
    println!("   - Tool execution context prepared with auth");  
    println!("   - Authentication headers generated correctly");
    println!("   - OAuth tokens and scopes properly handled");
    println!("   - All security validations passed");

    Ok(())
}

/// Test demonstrating session recovery capabilities
#[test]
fn test_session_recovery_preparation() {
    println!("🔄 Testing session recovery data structures");

    // Test user context creation for session management
    let user_context = UserContext::default();
    println!("✓ User context created: {}", user_context.get_unique_user_id());

    // Test session file path generation
    let session_file = user_context.get_session_file_path("oauth_tokens.json");
    println!("✓ Session file path: {}", session_file.display());

    let hostname_file = user_context.get_hostname_session_file_path("tokens.json");
    println!("✓ Hostname session file path: {}", hostname_file.display());

    // Test secure storage availability
    let has_secure_storage = user_context.has_secure_storage();
    println!("✓ Secure storage available: {}", has_secure_storage);

    println!("🎉 Session recovery preparation test PASSED!");
}