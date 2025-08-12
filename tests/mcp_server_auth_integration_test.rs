use magictunnel::auth::{MultiLevelAuthConfig, AuthMethod, OAuthProviderConfig, ApiKeyEntry, ServiceAccountConfig, ServiceAccountType};
use magictunnel::config::{Config, RegistryConfig};
use magictunnel::mcp::server::McpServer;
use magictunnel::error::Result;

#[tokio::test]
async fn test_mcp_server_with_multi_level_auth() -> Result<()> {
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
        ApiKeyEntry::new("admin_key".to_string(), "Admin Key".to_string(), "admin_secret".to_string())
    );
    
    // Add service account
    multi_level_auth.service_accounts.insert(
        "github_bot".to_string(),
        ServiceAccountConfig::github_pat("github_pat_token".to_string())
    );
    
    // Set hierarchical auth
    multi_level_auth.server_level = Some(AuthMethod::OAuth {
        provider: "github".to_string(),
        scopes: vec!["user:email".to_string()],
    });
    
    multi_level_auth.capabilities.insert(
        "github".to_string(),
        AuthMethod::ApiKey { key_ref: "admin_key".to_string() }
    );
    
    multi_level_auth.tools.insert(
        "github_create_issue".to_string(),
        AuthMethod::ServiceAccount { account_ref: "github_bot".to_string() }
    );
    
    // Create config with multi-level auth
    let mut config = Config::default();
    config.multi_level_auth = Some(multi_level_auth);
    
    // Validate the config
    config.validate()?;
    
    // Create MCP server with registry
    let registry_config = RegistryConfig::default();
    let server = McpServer::new(registry_config).await?;
    
    // Configure authentication from config
    let server = server.with_config_authentication(&config)?;
    
    // Test that the server was configured correctly
    // Note: In a real integration, we would need access to the server's internal state
    // For now, we're testing that the configuration and server setup doesn't fail
    
    println!("MCP Server with multi-level authentication configured successfully");
    println!("Config auth type: {}", config.get_auth_type_description());
    
    Ok(())
}

#[tokio::test]
async fn test_mcp_server_with_legacy_auth() -> Result<()> {
    // Test that legacy auth still works
    let mut config = Config::default();
    config.auth = Some(magictunnel::config::AuthConfig {
        enabled: true,
        r#type: magictunnel::config::AuthType::ApiKey,
        api_keys: Some(magictunnel::config::ApiKeyConfig {
            require_header: true,
            header_name: "X-API-Key".to_string(),
            header_format: "{key}".to_string(),
            keys: vec![magictunnel::config::ApiKeyEntry {
                name: "test".to_string(),
                description: Some("Test key".to_string()),
                key: "test_key_1234567890abcdef".to_string(),
                permissions: vec!["read".to_string()],
                active: true,
                expires_at: None,
            }],
        }),
        oauth: None,
        jwt: None,
    });
    
    // Validate the config
    config.validate()?;
    
    // Create MCP server
    let registry_config = RegistryConfig::default();
    let server = McpServer::new(registry_config).await?;
    
    // Configure with legacy authentication
    let server = server.with_config_authentication(&config)?;
    
    println!("MCP Server with legacy authentication configured successfully");
    println!("Config auth type: {}", config.get_auth_type_description());
    
    Ok(())
}

#[tokio::test]
async fn test_mcp_server_no_auth() -> Result<()> {
    // Test server with no authentication
    let config = Config::default();
    
    // Validate the config
    config.validate()?;
    
    // Create MCP server
    let registry_config = RegistryConfig::default();
    let server = McpServer::new(registry_config).await?;
    
    // Configure with no authentication
    let server = server.with_config_authentication(&config)?;
    
    println!("MCP Server with no authentication configured successfully");
    println!("Config auth type: {}", config.get_auth_type_description());
    
    assert_eq!(config.get_auth_type_description(), "Disabled");
    assert!(!config.is_auth_enabled());
    assert!(!config.is_multi_level_auth_enabled());
    assert!(!config.is_legacy_auth_enabled());
    
    Ok(())
}

#[tokio::test]
async fn test_auth_resolver_from_config() -> Result<()> {
    // Test that we can get an auth resolver from the config
    let mut multi_level_auth = MultiLevelAuthConfig::new();
    multi_level_auth.enabled = true;
    
    multi_level_auth.oauth_providers.insert(
        "github".to_string(),
        OAuthProviderConfig::github("id".to_string(), "secret".to_string())
    );
    
    multi_level_auth.server_level = Some(AuthMethod::OAuth {
        provider: "github".to_string(),
        scopes: vec!["user:email".to_string()],
    });
    
    let mut config = Config::default();
    config.multi_level_auth = Some(multi_level_auth);
    
    // Get resolver from config
    let resolver = config.get_auth_resolver()?;
    assert!(resolver.is_some());
    
    let resolver = resolver.unwrap();
    
    // Test that it resolves auth correctly
    assert!(resolver.requires_auth("some_tool"));
    
    let auth_method = resolver.resolve_auth_for_tool("some_tool");
    assert!(auth_method.is_some());
    
    match auth_method.unwrap() {
        AuthMethod::OAuth { provider, scopes } => {
            assert_eq!(provider, "github");
            assert_eq!(scopes, vec!["user:email"]);
        }
        _ => panic!("Expected OAuth auth method"),
    }
    
    // Test auth stats
    let stats = resolver.get_auth_stats();
    assert!(stats.enabled);
    assert_eq!(stats.total_oauth_providers, 1);
    assert!(stats.server_level_auth);
    
    Ok(())
}

#[tokio::test]
async fn test_config_validation_conflicts() -> Result<()> {
    // Test that conflicting auth configurations are caught
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
async fn test_yaml_roundtrip_integration() -> Result<()> {
    // Test that multi-level auth can be serialized to and from YAML
    let mut multi_level_auth = MultiLevelAuthConfig::new();
    multi_level_auth.enabled = true;
    
    multi_level_auth.oauth_providers.insert(
        "google".to_string(),
        OAuthProviderConfig::google("google_id".to_string(), "google_secret".to_string())
    );
    
    multi_level_auth.api_keys.push(
        ApiKeyEntry::new("key1".to_string(), "Key 1".to_string(), "secret1".to_string())
    );
    
    multi_level_auth.service_accounts.insert(
        "service1".to_string(),
        ServiceAccountConfig::new(ServiceAccountType::PersonalAccessToken, "token123".to_string())
    );
    
    multi_level_auth.server_level = Some(AuthMethod::DeviceCode {
        provider: "google".to_string(),
        scopes: vec!["openid".to_string(), "email".to_string()],
    });
    
    let mut config = Config::default();
    config.multi_level_auth = Some(multi_level_auth);
    
    // Serialize to YAML
    let yaml_str = serde_yaml::to_string(&config).expect("Failed to serialize to YAML");
    
    // Deserialize from YAML
    let deserialized_config: Config = serde_yaml::from_str(&yaml_str).expect("Failed to deserialize from YAML");
    
    // Validate deserialized config
    deserialized_config.validate()?;
    
    // Test that it has the same properties
    assert!(deserialized_config.is_multi_level_auth_enabled());
    assert!(!deserialized_config.is_legacy_auth_enabled());
    
    let resolver = deserialized_config.get_auth_resolver()?.unwrap();
    let stats = resolver.get_auth_stats();
    
    assert!(stats.enabled);
    assert_eq!(stats.total_oauth_providers, 1);
    assert_eq!(stats.total_api_keys, 1);
    assert_eq!(stats.total_service_accounts, 1);
    assert!(stats.server_level_auth);
    
    Ok(())
}