//! Authentication module for MCP Proxy
//!
//! This module provides authentication middleware and utilities for securing
//! MCP proxy endpoints with API key, OAuth, Device Code Flow, and JWT authentication.
//! 
//! The module supports multi-level authentication configuration at Server/Instance → 
//! Capability → Tool levels with hierarchical resolution.

pub mod api_key;
pub mod auth_context;
pub mod client_identity_extractor;
pub mod config;
pub mod device_code;
pub mod jwt;
pub mod middleware;
pub mod oauth;
pub mod oauth_providers;
pub mod provider_manager;
pub mod oauth_integration;
pub mod remote_session_middleware;
pub mod remote_token_storage;
pub mod remote_user_context;
pub mod resolver;
pub mod security_validator;
pub mod service_account;
pub mod session_isolation;
pub mod session_manager;
pub mod token_refresh;
pub mod token_storage;
pub mod user_context;

#[cfg(test)]
pub mod test_helpers;

pub use api_key::*;
pub use auth_context::{AuthenticationContext, ToolExecutionContext, ProviderToken};
pub use auth_context::AuthMethod as AuthContextMethod;
pub use client_identity_extractor::*;
pub use config::*;
pub use device_code::{
    DeviceCodeFlow, DeviceAuthorizationResponse, DeviceCodeValidator, 
    DeviceCodeValidationResult, DeviceCodeUserInfo, DeviceTokenResponse
};
pub use jwt::*;
pub use middleware::*;
pub use oauth::*;
pub use remote_session_middleware::*;
pub use remote_token_storage::*;
pub use remote_user_context::*;
pub use resolver::*;
pub use service_account::*;
pub use session_isolation::*;
pub use token_storage::*;
pub use user_context::*;
pub use provider_manager::{ProviderManager, ProviderManagerConfig, OAuthSession, ProviderStats};
pub use oauth_integration::UnifiedOAuthSystem;
