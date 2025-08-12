use magictunnel::auth::{MultiLevelAuthConfig, AuthMethod, AuthResolver, OAuthProviderConfig, ApiKeyEntry, ServiceAccountConfig, ServiceAccountType};
use magictunnel::config::Config;
use magictunnel::error::Result;
use serde_json;
use std::collections::HashMap;

#[tokio::test]
async fn test_multi_level_auth_config_integration() -> Result<()> {
    // Test 1: Default config has no auth
    let default_config = Config::default();
    assert!(default_config.multi_level_auth.is_none());
    assert!(!default_config.is_auth_enabled());
    assert!(!default_config.is_multi_level_auth_enabled());
    assert_eq!(default_config.get_auth_type_description(), "Disabled");

    // Test 2: Create config with multi-level auth
    let mut config = Config::default();
    let mut multi_level_auth = MultiLevelAuthConfig::new();
    multi_level_auth.enabled = true;
    
    // Add OAuth provider
    multi_level_auth.oauth_providers.insert(
        "github".to_string(),
        OAuthProviderConfig::github("client_id".to_string(), "client_secret".to_string())
    );
    
    // Add API key
    multi_level_auth.api_keys.push(
        ApiKeyEntry::new("test_key".to_string(), "Test Key".to_string(), "secret_value".to_string())
    );
    
    // Set server-level auth
    multi_level_auth.server_level = Some(AuthMethod::OAuth {
        provider: "github".to_string(),
        scopes: vec!["user:email".to_string()],
    });
    
    config.multi_level_auth = Some(multi_level_auth);
    
    // Test validation
    config.validate()?;
    
    // Test helper methods
    assert!(config.is_auth_enabled());
    assert!(config.is_multi_level_auth_enabled());
    assert!(!config.is_legacy_auth_enabled());
    
    let auth_description = config.get_auth_type_description();
    assert!(auth_description.contains("Multi-level auth"));
    assert!(auth_description.contains("server-level"));
    assert!(auth_description.contains("OAuth 2.1"));
    assert!(auth_description.contains("API keys"));

    Ok(())
}

#[tokio::test]
async fn test_auth_resolver_integration() -> Result<()> {
    // Create config with comprehensive auth setup
    let mut config = Config::default();
    let mut multi_level_auth = MultiLevelAuthConfig::new();
    multi_level_auth.enabled = true;
    
    // Add OAuth provider
    multi_level_auth.oauth_providers.insert(
        "github".to_string(),
        OAuthProviderConfig::github("client_id".to_string(), "client_secret".to_string())
    );
    
    // Add API key
    multi_level_auth.api_keys.push(
        ApiKeyEntry::new("api_key_1".to_string(), "API Key 1".to_string(), "secret_value".to_string())
    );
    
    // Add service account
    multi_level_auth.service_accounts.insert(
        "service_account_1".to_string(),
        ServiceAccountConfig::new(ServiceAccountType::PersonalAccessToken, "token_value".to_string())
    );
    
    // Set different auth levels
    multi_level_auth.server_level = Some(AuthMethod::OAuth {
        provider: "github".to_string(),
        scopes: vec!["user:email".to_string()],
    });
    
    multi_level_auth.capabilities.insert(
        "github".to_string(),
        AuthMethod::ApiKey { key_ref: "api_key_1".to_string() }
    );
    
    multi_level_auth.tools.insert(
        "github_create_issue".to_string(),
        AuthMethod::ServiceAccount { account_ref: "service_account_1".to_string() }
    );
    
    config.multi_level_auth = Some(multi_level_auth);
    
    // Test getting resolver
    let resolver = config.get_auth_resolver()?;
    assert!(resolver.is_some());
    let resolver = resolver.unwrap();
    
    // Test hierarchical resolution
    // Tool-specific auth (highest priority)
    let auth = resolver.resolve_auth_for_tool("github_create_issue");
    assert!(auth.is_some());
    match auth.unwrap() {
        AuthMethod::ServiceAccount { account_ref } => {
            assert_eq!(account_ref, "service_account_1");
        }
        _ => panic!("Expected ServiceAccount auth method"),
    }
    
    // Capability-level auth (medium priority)
    let auth = resolver.resolve_auth_for_tool("github.list_repos");
    assert!(auth.is_some());
    match auth.unwrap() {
        AuthMethod::ApiKey { key_ref } => {
            assert_eq!(key_ref, "api_key_1");
        }
        _ => panic!("Expected ApiKey auth method"),
    }
    
    // Server-level auth (lowest priority)
    let auth = resolver.resolve_auth_for_tool("some_other_tool");
    assert!(auth.is_some());
    match auth.unwrap() {
        AuthMethod::OAuth { provider, scopes } => {
            assert_eq!(provider, "github");
            assert_eq!(scopes, vec!["user:email"]);
        }
        _ => panic!("Expected OAuth auth method"),
    }

    Ok(())
}

#[tokio::test]
async fn test_conflicting_auth_configs() -> Result<()> {
    let mut config = Config::default();
    
    // Set both legacy and multi-level auth
    config.auth = Some(magictunnel::config::AuthConfig::default());
    config.multi_level_auth = Some(MultiLevelAuthConfig::new());
    
    // This should fail validation
    let result = config.validate();
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Cannot enable both legacy 'auth' and new 'multi_level_auth'"));

    Ok(())
}

#[tokio::test]
async fn test_multi_level_auth_yaml_serialization() -> Result<()> {
    // Create a comprehensive multi-level auth config
    let mut multi_level_auth = MultiLevelAuthConfig::new();
    multi_level_auth.enabled = true;
    
    // Add OAuth provider
    multi_level_auth.oauth_providers.insert(
        "github".to_string(),
        OAuthProviderConfig::github("test_client_id".to_string(), "test_client_secret".to_string())
    );
    
    // Add API key
    multi_level_auth.api_keys.push(
        ApiKeyEntry::with_rbac(
            "admin_key".to_string(),
            "Admin API Key".to_string(),
            "admin_secret".to_string(),
            "admin_user".to_string(),
            vec!["admin".to_string(), "write".to_string()]
        )
    );
    
    // Add service account
    multi_level_auth.service_accounts.insert(
        "github_pat".to_string(),
        ServiceAccountConfig::github_pat("github_pat_token".to_string())
    );
    
    // Set auth methods at different levels
    multi_level_auth.server_level = Some(AuthMethod::OAuth {
        provider: "github".to_string(),
        scopes: vec!["user:email".to_string(), "repo".to_string()],
    });
    
    multi_level_auth.capabilities.insert(
        "github".to_string(),
        AuthMethod::DeviceCode {
            provider: "github".to_string(),
            scopes: vec!["repo".to_string()],
        }
    );
    
    multi_level_auth.tools.insert(
        "github_create_issue".to_string(),
        AuthMethod::ServiceAccount { account_ref: "github_pat".to_string() }
    );
    
    // Test YAML serialization
    let yaml_str = serde_yaml::to_string(&multi_level_auth).expect("Failed to serialize to YAML");
    
    // Test YAML deserialization
    let deserialized: MultiLevelAuthConfig = serde_yaml::from_str(&yaml_str).expect("Failed to deserialize from YAML");
    
    // Verify deserialized config
    assert_eq!(deserialized.enabled, true);
    assert!(deserialized.oauth_providers.contains_key("github"));
    assert_eq!(deserialized.api_keys.len(), 1);
    assert!(deserialized.service_accounts.contains_key("github_pat"));
    assert!(deserialized.server_level.is_some());
    assert!(deserialized.capabilities.contains_key("github"));
    assert!(deserialized.tools.contains_key("github_create_issue"));
    
    // Test validation of deserialized config
    deserialized.validate()?;

    Ok(())
}

#[tokio::test]
async fn test_auth_validation_errors() -> Result<()> {
    let mut config = Config::default();
    let mut multi_level_auth = MultiLevelAuthConfig::new();
    multi_level_auth.enabled = true;
    
    // Test invalid OAuth provider reference
    multi_level_auth.server_level = Some(AuthMethod::OAuth {
        provider: "nonexistent".to_string(),
        scopes: vec!["user:email".to_string()],
    });
    
    config.multi_level_auth = Some(multi_level_auth.clone());
    
    let result = config.validate();
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("references unknown OAuth provider"));
    
    // Reset and test invalid API key reference
    multi_level_auth.server_level = Some(AuthMethod::ApiKey {
        key_ref: "nonexistent_key".to_string(),
    });
    
    config.multi_level_auth = Some(multi_level_auth.clone());
    
    let result = config.validate();
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("references unknown API key"));
    
    // Reset and test invalid service account reference
    multi_level_auth.server_level = Some(AuthMethod::ServiceAccount {
        account_ref: "nonexistent_account".to_string(),
    });
    
    config.multi_level_auth = Some(multi_level_auth);
    
    let result = config.validate();
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("references unknown service account"));

    Ok(())
}

#[tokio::test]
async fn test_complete_config_integration() -> Result<()> {
    // Test a complete configuration that includes multi-level auth alongside other features
    let mut config = Config::default();
    
    // Set up multi-level auth
    let mut multi_level_auth = MultiLevelAuthConfig::new();
    multi_level_auth.enabled = true;
    
    // Add comprehensive auth setup
    multi_level_auth.oauth_providers.insert(
        "github".to_string(),
        OAuthProviderConfig::github("id".to_string(), "secret".to_string())
    );
    multi_level_auth.oauth_providers.insert(
        "google".to_string(),
        OAuthProviderConfig::google("google_id".to_string(), "google_secret".to_string())
    );
    
    multi_level_auth.api_keys.push(
        ApiKeyEntry::new("key1".to_string(), "Key 1".to_string(), "secret1".to_string())
    );
    multi_level_auth.api_keys.push(
        ApiKeyEntry::new("key2".to_string(), "Key 2".to_string(), "secret2".to_string())
    );
    
    multi_level_auth.service_accounts.insert(
        "github_bot".to_string(),
        ServiceAccountConfig::github_pat("bot_token".to_string())
    );
    multi_level_auth.service_accounts.insert(
        "google_service".to_string(),
        ServiceAccountConfig::google_service_key("service_key_json".to_string())
    );
    
    // Set up hierarchical auth
    multi_level_auth.server_level = Some(AuthMethod::OAuth {
        provider: "github".to_string(),
        scopes: vec!["user:email".to_string()],
    });
    
    multi_level_auth.capabilities.insert("github".to_string(), AuthMethod::ApiKey { key_ref: "key1".to_string() });
    multi_level_auth.capabilities.insert("google".to_string(), AuthMethod::OAuth { provider: "google".to_string(), scopes: vec!["openid".to_string()] });
    
    multi_level_auth.tools.insert("github.create_issue".to_string(), AuthMethod::ServiceAccount { account_ref: "github_bot".to_string() });
    multi_level_auth.tools.insert("google.gmail_send".to_string(), AuthMethod::ServiceAccount { account_ref: "google_service".to_string() });
    
    config.multi_level_auth = Some(multi_level_auth);
    
    // Test that the complete config validates
    config.validate()?;
    
    // Test that all helper methods work correctly
    assert!(config.is_auth_enabled());
    assert!(config.is_multi_level_auth_enabled());
    assert!(!config.is_legacy_auth_enabled());
    
    let auth_description = config.get_auth_type_description();
    assert!(auth_description.contains("Multi-level auth"));
    assert!(auth_description.contains("server-level"));
    assert!(auth_description.contains("capability-level"));
    assert!(auth_description.contains("tool-level"));
    assert!(auth_description.contains("OAuth 2.1"));
    assert!(auth_description.contains("API keys"));
    assert!(auth_description.contains("service accounts"));
    
    // Test auth resolver functionality
    let resolver = config.get_auth_resolver()?.unwrap();
    
    // Test comprehensive resolution patterns
    assert!(resolver.requires_auth("github.create_issue"));
    assert!(resolver.requires_auth("github.list_repos"));
    assert!(resolver.requires_auth("google.gmail_send"));
    assert!(resolver.requires_auth("random_tool"));
    
    // Test capability-level resolution
    assert!(resolver.capability_requires_auth("github"));
    assert!(resolver.capability_requires_auth("google"));
    // Even nonauth_service requires auth due to server-level auth
    assert!(resolver.capability_requires_auth("nonauth_service"));
    
    // Test auth method retrieval
    let github_issue_auth = resolver.resolve_auth_for_tool("github.create_issue").unwrap();
    match github_issue_auth {
        AuthMethod::ServiceAccount { account_ref } => assert_eq!(account_ref, "github_bot"),
        _ => panic!("Expected ServiceAccount for github.create_issue"),
    }
    
    let github_general_auth = resolver.resolve_auth_for_tool("github.list_repos").unwrap();
    match github_general_auth {
        AuthMethod::ApiKey { key_ref } => assert_eq!(key_ref, "key1"),
        _ => panic!("Expected ApiKey for github capability"),
    }
    
    let google_gmail_auth = resolver.resolve_auth_for_tool("google.gmail_send").unwrap();
    match google_gmail_auth {
        AuthMethod::ServiceAccount { account_ref } => assert_eq!(account_ref, "google_service"),
        _ => panic!("Expected ServiceAccount for google.gmail_send"),
    }
    
    let random_tool_auth = resolver.resolve_auth_for_tool("random_tool").unwrap();
    match random_tool_auth {
        AuthMethod::OAuth { provider, scopes } => {
            assert_eq!(provider, "github");
            assert_eq!(scopes, vec!["user:email"]);
        },
        _ => panic!("Expected OAuth for server-level auth"),
    }

    Ok(())
}

#[tokio::test]
async fn test_auth_stats_integration() -> Result<()> {
    let mut config = Config::default();
    let mut multi_level_auth = MultiLevelAuthConfig::new();
    multi_level_auth.enabled = true;
    
    // Set up auth config with various methods
    multi_level_auth.oauth_providers.insert("github".to_string(), OAuthProviderConfig::github("id".to_string(), "secret".to_string()));
    multi_level_auth.oauth_providers.insert("google".to_string(), OAuthProviderConfig::google("id".to_string(), "secret".to_string()));
    
    multi_level_auth.api_keys.push(ApiKeyEntry::new("key1".to_string(), "Key 1".to_string(), "secret".to_string()));
    multi_level_auth.api_keys.push(ApiKeyEntry::new("key2".to_string(), "Key 2".to_string(), "secret".to_string()));
    multi_level_auth.api_keys.push(ApiKeyEntry::new("key3".to_string(), "Key 3".to_string(), "secret".to_string()));
    
    multi_level_auth.service_accounts.insert("account1".to_string(), ServiceAccountConfig::github_pat("token".to_string()));
    
    multi_level_auth.server_level = Some(AuthMethod::OAuth { provider: "github".to_string(), scopes: vec![] });
    multi_level_auth.capabilities.insert("cap1".to_string(), AuthMethod::ApiKey { key_ref: "key1".to_string() });
    multi_level_auth.capabilities.insert("cap2".to_string(), AuthMethod::DeviceCode { provider: "google".to_string(), scopes: vec![] });
    multi_level_auth.tools.insert("tool1".to_string(), AuthMethod::ServiceAccount { account_ref: "account1".to_string() });
    multi_level_auth.tools.insert("tool2".to_string(), AuthMethod::OAuth { provider: "google".to_string(), scopes: vec![] });
    
    config.multi_level_auth = Some(multi_level_auth);
    
    let resolver = config.get_auth_resolver()?.unwrap();
    let stats = resolver.get_auth_stats();
    
    assert!(stats.enabled);
    assert_eq!(stats.total_oauth_providers, 2);
    assert_eq!(stats.total_api_keys, 3);
    assert_eq!(stats.total_service_accounts, 1);
    assert!(stats.server_level_auth);
    assert_eq!(stats.capability_level_auths, 2);
    assert_eq!(stats.tool_level_auths, 2);
    
    // Check auth method distribution
    assert_eq!(stats.auth_method_distribution.get("oauth"), Some(&2)); // server + tool2
    assert_eq!(stats.auth_method_distribution.get("device_code"), Some(&1)); // cap2
    assert_eq!(stats.auth_method_distribution.get("api_key"), Some(&1)); // cap1
    assert_eq!(stats.auth_method_distribution.get("service_account"), Some(&1)); // tool1

    Ok(())
}

#[tokio::test]
async fn test_disabled_multi_level_auth() -> Result<()> {
    let mut config = Config::default();
    let mut multi_level_auth = MultiLevelAuthConfig::new();
    multi_level_auth.enabled = false; // Disabled auth
    
    // Even with auth methods configured, they should be ignored if disabled
    multi_level_auth.oauth_providers.insert("github".to_string(), OAuthProviderConfig::github("id".to_string(), "secret".to_string()));
    multi_level_auth.server_level = Some(AuthMethod::OAuth { provider: "github".to_string(), scopes: vec![] });
    
    config.multi_level_auth = Some(multi_level_auth);
    
    // Should validate fine
    config.validate()?;
    
    // Auth should be considered disabled
    assert!(!config.is_auth_enabled());
    assert!(!config.is_multi_level_auth_enabled());
    assert_eq!(config.get_auth_type_description(), "Disabled");
    
    // Resolver should return None for all tools
    let resolver = config.get_auth_resolver()?.unwrap();
    assert!(!resolver.requires_auth("any_tool"));
    assert!(resolver.resolve_auth_for_tool("any_tool").is_none());

    Ok(())
}