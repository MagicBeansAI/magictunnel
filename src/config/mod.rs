//! Configuration module for MCP Proxy
//! 
//! This module provides configuration management and loading utilities.

mod config;

// Re-export the main configuration types
pub use config::{
    Config, ServerConfig, RegistryConfig, AuthConfig, LoggingConfig, ValidationConfig, OAuthConfig,
    ConflictResolutionStrategy, AggregationConfig, VisibilityConfig,
    // Authentication types
    AuthType, ApiKeyConfig, ApiKeyEntry, JwtConfig,
    // TLS types
    TlsConfig, TlsMode,
    // MCP Client types
    McpClientConfig,
    // External MCP types (unified local/remote)
    ExternalMcpConfig, ContainerConfig, McpServerConfig, ExternalMcpServersConfig
};
