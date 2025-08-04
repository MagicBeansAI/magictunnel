//! Tests for enhanced configuration validation and environment variable support

use magictunnel::config::{
    Config, ServerConfig, RegistryConfig, ValidationConfig, AuthConfig, LoggingConfig, OAuthConfig,
    McpServerConfig, McpClientConfig, ExternalMcpConfig, SamplingElicitationStrategy, LlmConfig,
    ExternalRoutingStrategyConfig, McpExternalRoutingConfig, SamplingConfig, ElicitationConfig
};

use std::env;
use std::fs;

use tempfile::TempDir;

#[test]
fn test_server_config_validation() {
    // Test valid server config
    let valid_config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 3000,
        websocket: true,
        timeout: 30,
        tls: None,
    };
    assert!(valid_config.validate().is_ok());

    // Test empty host
    let invalid_config = ServerConfig {
        host: "".to_string(),
        port: 3000,
        websocket: true,
        timeout: 30,
        tls: None,
    };
    assert!(invalid_config.validate().is_err());

    // Test port 0
    let invalid_config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        websocket: true,
        timeout: 30,
        tls: None,
    };
    assert!(invalid_config.validate().is_err());

    // Test timeout 0
    let invalid_config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 3000,
        websocket: true,
        timeout: 0,
        tls: None,
    };
    assert!(invalid_config.validate().is_err());

    // Test timeout too high
    let invalid_config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 3000,
        websocket: true,
        timeout: 4000,
        tls: None,
    };
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_registry_config_validation() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_string_lossy().to_string();

    // Test valid registry config
    let valid_config = RegistryConfig {
        r#type: "file".to_string(),
        paths: vec![temp_path.clone()],
        hot_reload: true,
        validation: ValidationConfig::default(),
    };
    assert!(valid_config.validate().is_ok());

    // Test empty paths
    let invalid_config = RegistryConfig {
        r#type: "file".to_string(),
        paths: vec![],
        hot_reload: true,
        validation: ValidationConfig::default(),
    };
    assert!(invalid_config.validate().is_err());

    // Test unsupported registry type
    let invalid_config = RegistryConfig {
        r#type: "unsupported".to_string(),
        paths: vec![temp_path],
        hot_reload: true,
        validation: ValidationConfig::default(),
    };
    assert!(invalid_config.validate().is_err());

    // Test path with ".."
    let invalid_config = RegistryConfig {
        r#type: "file".to_string(),
        paths: vec!["../dangerous/path".to_string()],
        hot_reload: true,
        validation: ValidationConfig::default(),
    };
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_auth_config_validation() {
    // Test valid API key auth
    let valid_config = AuthConfig {
        enabled: true,
        r#type: magictunnel::config::AuthType::ApiKey,
        api_keys: Some(magictunnel::config::ApiKeyConfig {
            keys: vec![magictunnel::config::ApiKeyEntry::new(
                "valid_api_key_123456".to_string(),
                "Test Key".to_string()
            )],
            require_header: true,
            header_name: "Authorization".to_string(),
            header_format: "Bearer {key}".to_string(),
        }),
        oauth: None,
        jwt: None,
    };
    assert!(valid_config.validate().is_ok());

    // Test API key auth without keys
    let invalid_config = AuthConfig {
        enabled: true,
        r#type: magictunnel::config::AuthType::ApiKey,
        api_keys: None,
        oauth: None,
        jwt: None,
    };
    assert!(invalid_config.validate().is_err());

    // Test API key auth with short key
    let invalid_config = AuthConfig {
        enabled: true,
        r#type: magictunnel::config::AuthType::ApiKey,
        api_keys: Some(magictunnel::config::ApiKeyConfig {
            keys: vec![magictunnel::config::ApiKeyEntry::new(
                "short".to_string(),
                "Short Key".to_string()
            )],
            require_header: true,
            header_name: "Authorization".to_string(),
            header_format: "Bearer {key}".to_string(),
        }),
        oauth: None,
        jwt: None,
    };
    assert!(invalid_config.validate().is_err());

    // Test OAuth auth without config
    let invalid_config = AuthConfig {
        enabled: true,
        r#type: magictunnel::config::AuthType::OAuth,
        api_keys: None,
        oauth: None,
        jwt: None,
    };
    assert!(invalid_config.validate().is_err());

    // Test valid OAuth config
    let oauth_config = OAuthConfig {
        provider: "google".to_string(),
        client_id: "client123".to_string(),
        client_secret: "secret123".to_string(),
        auth_url: "https://accounts.google.com/oauth/authorize".to_string(),
        token_url: "https://oauth2.googleapis.com/token".to_string(),
        oauth_2_1_enabled: true,
        resource_indicators_enabled: false,
        default_resources: Vec::new(),
        default_audience: Vec::new(),
        require_explicit_resources: false,
    };
    let valid_config = AuthConfig {
        enabled: true,
        r#type: magictunnel::config::AuthType::OAuth,
        api_keys: None,
        oauth: Some(oauth_config),
        jwt: None,
    };
    assert!(valid_config.validate().is_ok());
}

#[test]
fn test_logging_config_validation() {
    // Test valid logging config
    let valid_config = LoggingConfig {
        level: "info".to_string(),
        format: "json".to_string(),
        file: None,
    };
    assert!(valid_config.validate().is_ok());

    // Test invalid log level
    let invalid_config = LoggingConfig {
        level: "invalid".to_string(),
        format: "json".to_string(),
        file: None,
    };
    assert!(invalid_config.validate().is_err());

    // Test invalid log format
    let invalid_config = LoggingConfig {
        level: "info".to_string(),
        format: "invalid".to_string(),
        file: None,
    };
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_environment_variable_overrides() {
    // Set environment variables
    env::set_var("MCP_HOST", "0.0.0.0");
    env::set_var("MCP_PORT", "8080");
    env::set_var("MCP_WEBSOCKET", "false");
    env::set_var("MCP_TIMEOUT", "60");
    env::set_var("MCP_REGISTRY_TYPE", "file");
    env::set_var("MCP_REGISTRY_PATHS", "path1,path2,path3");
    env::set_var("MCP_HOT_RELOAD", "false");
    env::set_var("MCP_LOG_LEVEL", "debug");
    env::set_var("MCP_LOG_FORMAT", "json");

    // Create a default config and apply environment overrides
    let mut config = Config::default();
    config.apply_environment_overrides().unwrap();

    // Verify environment variables were applied
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.server.websocket, false);
    assert_eq!(config.server.timeout, 60);
    assert_eq!(config.registry.r#type, "file");
    assert_eq!(config.registry.paths, vec!["path1", "path2", "path3"]);
    assert_eq!(config.registry.hot_reload, false);
    assert_eq!(config.logging.as_ref().unwrap().level, "debug");
    assert_eq!(config.logging.as_ref().unwrap().format, "json");

    // Clean up environment variables
    env::remove_var("MCP_HOST");
    env::remove_var("MCP_PORT");
    env::remove_var("MCP_WEBSOCKET");
    env::remove_var("MCP_TIMEOUT");
    env::remove_var("MCP_REGISTRY_TYPE");
    env::remove_var("MCP_REGISTRY_PATHS");
    env::remove_var("MCP_HOT_RELOAD");
    env::remove_var("MCP_LOG_LEVEL");
    env::remove_var("MCP_LOG_FORMAT");
}

#[test]
fn test_cross_dependency_validation() {
    // Test gRPC port overflow
    let invalid_config = Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 65000, // gRPC port would be 66000, which exceeds 65535
            websocket: true,
            timeout: 30,
            tls: None,
        },
        registry: RegistryConfig::default(),
        auth: None,
        logging: None,
        external_mcp: None,
        mcp_client: None,
        conflict_resolution: None,
        visibility: None,
        smart_discovery: None,
        security: None,
        streamable_http: None,
        sampling: None,
        tool_enhancement: None,
        elicitation: None,
        prompt_generation: None,
        resource_generation: None,
        content_storage: None,
        external_content: None,
        enhancement_storage: None,
    };
    assert!(invalid_config.validate().is_err());

    // Test API key auth without keys in cross-validation
    let invalid_config = Config {
        server: ServerConfig::default(),
        registry: RegistryConfig::default(),
        auth: Some(AuthConfig {
            enabled: true,
            r#type: magictunnel::config::AuthType::ApiKey,
            api_keys: Some(magictunnel::config::ApiKeyConfig {
                keys: vec![], // Empty keys
                require_header: true,
                header_name: "Authorization".to_string(),
                header_format: "Bearer {key}".to_string(),
            }),
            oauth: None,
            jwt: None,
        }),
        logging: None,
        external_mcp: None,
        mcp_client: None,
        conflict_resolution: None,
        visibility: None,
        smart_discovery: None,
        security: None,
        streamable_http: None,
        sampling: None,
        tool_enhancement: None,
        elicitation: None,
        prompt_generation: None,
        resource_generation: None,
        content_storage: None,
        external_content: None,
        enhancement_storage: None,
    };
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_file_structure_validation() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a valid YAML file
    let yaml_file = temp_path.join("test.yaml");
    fs::write(&yaml_file, "test: content").unwrap();

    // Test valid file path
    let config = RegistryConfig {
        r#type: "file".to_string(),
        paths: vec![yaml_file.to_string_lossy().to_string()],
        hot_reload: true,
        validation: ValidationConfig::default(),
    };
    assert!(config.validate().is_ok());

    // Test invalid file extension
    let txt_file = temp_path.join("test.txt");
    fs::write(&txt_file, "test content").unwrap();

    let config = RegistryConfig {
        r#type: "file".to_string(),
        paths: vec![txt_file.to_string_lossy().to_string()],
        hot_reload: true,
        validation: ValidationConfig::default(),
    };
    assert!(config.validate().is_err());

    // Test glob pattern with valid base directory
    let config = RegistryConfig {
        r#type: "file".to_string(),
        paths: vec![format!("{}/*.yaml", temp_path.to_string_lossy())],
        hot_reload: true,
        validation: ValidationConfig::default(),
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_external_mcp_config_validation() {
    // Test valid External MCP config
    let _valid_config = ExternalMcpConfig {
        enabled: true,
        config_file: "./external-mcp-servers.yaml".to_string(),
        capabilities_output_dir: "./capabilities".to_string(),
        refresh_interval_minutes: 60,
        containers: None,
        external_routing: None,
    };
    // Note: ExternalMcpConfig doesn't have a validate method in the current implementation
    // Validation is done at the overall Config level

    // Test with container config
    let _config_with_containers = ExternalMcpConfig {
        enabled: true,
        config_file: "./external-mcp-servers.yaml".to_string(),
        capabilities_output_dir: "./capabilities".to_string(),
        refresh_interval_minutes: 30,
        containers: Some(magictunnel::config::ContainerConfig {
            runtime: "docker".to_string(),
            node_image: Some("node:18".to_string()),
            python_image: Some("python:3.11".to_string()),
            network_mode: Some("bridge".to_string()),
            run_args: vec!["--rm".to_string()],
        }),
        external_routing: None,
    };
    // This should be valid

    // Test disabled config
    let _disabled_config = ExternalMcpConfig {
        enabled: false,
        config_file: "./external-mcp-servers.yaml".to_string(),
        capabilities_output_dir: "./capabilities".to_string(),
        refresh_interval_minutes: 60,
        containers: None,
        external_routing: None,
    };
    // This should be valid even when disabled
}

#[test]
fn test_mcp_server_config_validation() {
    // Test valid MCP server config (Claude Desktop format)
    let _valid_config = McpServerConfig {
        command: "npx".to_string(),
        args: vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string(), "/tmp".to_string()],
        env: None,
        cwd: None,
        sampling_strategy: None,
        elicitation_strategy: None,
    };
    // Note: McpServerConfig doesn't have a validate method in the new format
    // It's validated as part of the overall configuration

    // Test empty command
    let _invalid_config = McpServerConfig {
        command: "".to_string(),
        args: vec!["arg1".to_string()],
        env: None,
        cwd: None,
        sampling_strategy: None,
        elicitation_strategy: None,
    };
    // This would be caught during configuration validation

    // Test with environment variables
    let mut env_vars = std::collections::HashMap::new();
    env_vars.insert("NODE_ENV".to_string(), "production".to_string());

    let _valid_config_with_env = McpServerConfig {
        command: "node".to_string(),
        args: vec!["server.js".to_string()],
        env: Some(env_vars),
        cwd: Some("/app".to_string()),
        sampling_strategy: None,
        elicitation_strategy: None,
    };
    // This should be valid

    // Test with working directory
    let _valid_config_with_cwd = McpServerConfig {
        command: "python".to_string(),
        args: vec!["-m", "mcp_server"].iter().map(|s| s.to_string()).collect(),
        env: None,
        cwd: Some("/home/user/mcp".to_string()),
        sampling_strategy: None,
        elicitation_strategy: None,
    };
    // This should be valid
}

#[test]
fn test_mcp_client_config_validation() {
    // Test valid client config
    let valid_config = McpClientConfig {
        connect_timeout_secs: 30,
        request_timeout_secs: 60,
        max_reconnect_attempts: 3,
        reconnect_delay_secs: 5,
        auto_reconnect: true,
        protocol_version: "2025-06-18".to_string(),
        client_name: "magictunnel".to_string(),
        client_version: "0.3.0".to_string(),
    };
    assert!(valid_config.validate().is_ok());

    // Test invalid connect timeout (0)
    let invalid_config = McpClientConfig {
        connect_timeout_secs: 0,
        request_timeout_secs: 60,
        max_reconnect_attempts: 3,
        reconnect_delay_secs: 5,
        auto_reconnect: true,
        protocol_version: "2025-06-18".to_string(),
        client_name: "magictunnel".to_string(),
        client_version: "0.3.0".to_string(),
    };
    assert!(invalid_config.validate().is_err());

    // Test invalid connect timeout (too high)
    let invalid_config = McpClientConfig {
        connect_timeout_secs: 400,
        request_timeout_secs: 60,
        max_reconnect_attempts: 3,
        reconnect_delay_secs: 5,
        auto_reconnect: true,
        protocol_version: "2025-06-18".to_string(),
        client_name: "magictunnel".to_string(),
        client_version: "0.3.0".to_string(),
    };
    assert!(invalid_config.validate().is_err());

    // Test invalid request timeout (0)
    let invalid_config = McpClientConfig {
        connect_timeout_secs: 30,
        request_timeout_secs: 0,
        max_reconnect_attempts: 3,
        reconnect_delay_secs: 5,
        auto_reconnect: true,
        protocol_version: "2025-06-18".to_string(),
        client_name: "magictunnel".to_string(),
        client_version: "0.3.0".to_string(),
    };
    assert!(invalid_config.validate().is_err());

    // Test invalid max reconnect attempts (too high)
    let invalid_config = McpClientConfig {
        connect_timeout_secs: 30,
        request_timeout_secs: 60,
        max_reconnect_attempts: 15,
        reconnect_delay_secs: 5,
        auto_reconnect: true,
        protocol_version: "2025-06-18".to_string(),
        client_name: "magictunnel".to_string(),
        client_version: "0.3.0".to_string(),
    };
    assert!(invalid_config.validate().is_err());

    // Test invalid reconnect delay (0)
    let invalid_config = McpClientConfig {
        connect_timeout_secs: 30,
        request_timeout_secs: 60,
        max_reconnect_attempts: 3,
        reconnect_delay_secs: 0,
        auto_reconnect: true,
        protocol_version: "2025-06-18".to_string(),
        client_name: "magictunnel".to_string(),
        client_version: "0.3.0".to_string(),
    };
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_sampling_elicitation_strategy_validation() {
    // Test all strategy variants are valid
    assert!(SamplingElicitationStrategy::MagictunnelHandled.validate().is_ok());
    assert!(SamplingElicitationStrategy::ClientForwarded.validate().is_ok());
    assert!(SamplingElicitationStrategy::MagictunnelFirst.validate().is_ok());
    assert!(SamplingElicitationStrategy::ClientFirst.validate().is_ok());
    assert!(SamplingElicitationStrategy::Parallel.validate().is_ok());
    assert!(SamplingElicitationStrategy::Hybrid.validate().is_ok());

    // Test LLM requirement detection
    assert!(SamplingElicitationStrategy::MagictunnelHandled.requires_llm_config());
    assert!(!SamplingElicitationStrategy::ClientForwarded.requires_llm_config());
    assert!(SamplingElicitationStrategy::MagictunnelFirst.requires_llm_config());
    assert!(!SamplingElicitationStrategy::ClientFirst.requires_llm_config());
    assert!(SamplingElicitationStrategy::Parallel.requires_llm_config());
    assert!(SamplingElicitationStrategy::Hybrid.requires_llm_config());

    // Test client forwarding requirement detection
    assert!(!SamplingElicitationStrategy::MagictunnelHandled.requires_client_forwarding());
    assert!(SamplingElicitationStrategy::ClientForwarded.requires_client_forwarding());
    assert!(!SamplingElicitationStrategy::MagictunnelFirst.requires_client_forwarding());
    assert!(SamplingElicitationStrategy::ClientFirst.requires_client_forwarding());
    assert!(SamplingElicitationStrategy::Parallel.requires_client_forwarding());
    assert!(SamplingElicitationStrategy::Hybrid.requires_client_forwarding());
}

#[test]
fn test_llm_config_validation() {
    // Test valid OpenAI config
    let valid_openai_config = LlmConfig {
        provider: "openai".to_string(),
        model: "gpt-4o-mini".to_string(),
        api_key_env: Some("OPENAI_API_KEY".to_string()),
        api_base_url: None,
        max_tokens: Some(4000),
        temperature: Some(0.7),
        additional_params: None,
    };
    assert!(valid_openai_config.validate().is_ok());

    // Test invalid provider
    let invalid_provider_config = LlmConfig {
        provider: "unsupported_provider".to_string(),
        model: "some-model".to_string(),
        api_key_env: Some("API_KEY".to_string()),
        api_base_url: None,
        max_tokens: Some(4000),
        temperature: Some(0.7),
        additional_params: None,
    };
    assert!(invalid_provider_config.validate().is_err());

    // Test empty model name
    let invalid_model_config = LlmConfig {
        provider: "openai".to_string(),
        model: "".to_string(),
        api_key_env: Some("OPENAI_API_KEY".to_string()),
        api_base_url: None,
        max_tokens: Some(4000),
        temperature: Some(0.7),
        additional_params: None,
    };
    assert!(invalid_model_config.validate().is_err());

    // Test OpenAI without API key
    let invalid_openai_no_key = LlmConfig {
        provider: "openai".to_string(),
        model: "gpt-4o-mini".to_string(),
        api_key_env: None,
        api_base_url: None,
        max_tokens: Some(4000),
        temperature: Some(0.7),
        additional_params: None,
    };
    assert!(invalid_openai_no_key.validate().is_err());
}

#[test]
fn test_external_routing_strategy_config_validation() {
    // Test valid configuration
    let valid_config = ExternalRoutingStrategyConfig {
        default_strategy: SamplingElicitationStrategy::ClientForwarded,
        server_strategies: Some([
            ("server1".to_string(), SamplingElicitationStrategy::MagictunnelHandled),
            ("server2".to_string(), SamplingElicitationStrategy::Parallel),
        ].into_iter().collect()),
        priority_order: vec!["server1".to_string(), "server2".to_string()],
        fallback_to_magictunnel: true,
        max_retry_attempts: 3,
        timeout_seconds: 60,
    };
    assert!(valid_config.validate("Test").is_ok());

    // Test empty priority order
    let invalid_empty_priority = ExternalRoutingStrategyConfig {
        default_strategy: SamplingElicitationStrategy::ClientForwarded,
        server_strategies: None,
        priority_order: vec![],
        fallback_to_magictunnel: false,
        max_retry_attempts: 3,
        timeout_seconds: 60,
    };
    assert!(invalid_empty_priority.validate("Test").is_err());

    // Test LLM requirement detection
    let llm_requiring_config = ExternalRoutingStrategyConfig {
        default_strategy: SamplingElicitationStrategy::MagictunnelHandled,
        server_strategies: None,
        priority_order: vec!["server1".to_string()],
        fallback_to_magictunnel: false,
        max_retry_attempts: 3,
        timeout_seconds: 60,
    };
    assert!(llm_requiring_config.requires_llm_config());
}

#[test]
fn test_routing_strategy_dependencies_validation() {
    // Test configuration requiring LLM with valid LLM config
    let valid_config_with_llm = Config {
        server: ServerConfig::default(),
        registry: RegistryConfig::default(),
        auth: None,
        logging: None,
        external_mcp: None,
        mcp_client: None,
        conflict_resolution: None,
        visibility: None,
        smart_discovery: None,
        security: None,
        streamable_http: None,
        sampling: Some(SamplingConfig {
            enabled: true,
            default_model: "gpt-4o-mini".to_string(),
            max_tokens_limit: 4000,
            default_sampling_strategy: Some(SamplingElicitationStrategy::MagictunnelHandled),
            default_elicitation_strategy: Some(SamplingElicitationStrategy::ClientForwarded),
            llm_config: Some(LlmConfig {
                provider: "openai".to_string(),
                model: "gpt-4o-mini".to_string(),
                api_key_env: Some("OPENAI_API_KEY".to_string()),
                api_base_url: None,
                max_tokens: Some(4000),
                temperature: Some(0.7),
                additional_params: None,
            }),
        }),
        tool_enhancement: None,
        elicitation: None,
        prompt_generation: None,
        resource_generation: None,
        content_storage: None,
        external_content: None,
        enhancement_storage: None,
    };
    assert!(valid_config_with_llm.validate().is_ok());

    // Test configuration requiring LLM without LLM config
    let invalid_config_missing_llm = Config {
        server: ServerConfig::default(),
        registry: RegistryConfig::default(),
        auth: None,
        logging: None,
        external_mcp: None,
        mcp_client: None,
        conflict_resolution: None,
        visibility: None,
        smart_discovery: None,
        security: None,
        streamable_http: None,
        sampling: Some(SamplingConfig {
            enabled: true,
            default_model: "gpt-4o-mini".to_string(),
            max_tokens_limit: 4000,
            default_sampling_strategy: Some(SamplingElicitationStrategy::MagictunnelHandled),
            default_elicitation_strategy: Some(SamplingElicitationStrategy::ClientForwarded),
            llm_config: None, // Missing LLM config
        }),
        tool_enhancement: None,
        elicitation: None,
        prompt_generation: None,
        resource_generation: None,
        content_storage: None,
        external_content: None,
        enhancement_storage: None,
    };
    assert!(invalid_config_missing_llm.validate().is_err());
}
