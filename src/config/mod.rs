//! Configuration module for MCP Proxy
//! 
//! This module provides configuration management and loading utilities.

mod config;
mod environment;
mod resolver;
mod validator;

// Re-export the main configuration types
pub use config::{
    Config, ServerConfig, RegistryConfig, AuthConfig, LoggingConfig, ValidationConfig, OAuthConfig,
    ConflictResolutionStrategy, AggregationConfig, VisibilityConfig,
    // Deployment types
    RuntimeMode, DeploymentConfig,
    // Authentication types
    AuthType, ApiKeyConfig, ApiKeyEntry, JwtConfig,
    // TLS types
    TlsConfig, TlsMode,
    // MCP Client types
    McpClientConfig,
    // External MCP types (unified local/remote)
    ExternalMcpConfig, ContainerConfig, McpServerConfig, ExternalMcpServersConfig,
    // Network MCP service types
    HttpServiceConfig, SseServiceConfig, WebSocketServiceConfig,
    HttpAuthType, SseAuthType, WebSocketAuthType,
    // MCP 2025-06-18 feature types
    SamplingConfig, ElicitationConfig, LlmConfig, SamplingElicitationStrategy,
    // MCP External Routing types (MCP 2025-06-18)
    McpExternalRoutingConfig, ExternalRoutingStrategyConfig
};

// Re-export environment and resolver types
pub use environment::{EnvironmentOverrides, EnvVars};
pub use resolver::{ConfigResolver, ConfigResolution, ConfigSource, ConfigStartupSummary};

// Re-export validator types
pub use validator::{ConfigValidator, ValidationResult, ConfigUpdateSummary, ConfigFixSuggestions, QuickFix};
