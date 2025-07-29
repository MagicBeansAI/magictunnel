//! Authentication module for MCP Proxy
//!
//! This module provides authentication middleware and utilities for securing
//! MCP proxy endpoints with API key, OAuth, and JWT authentication.

pub mod api_key;
pub mod jwt;
pub mod middleware;
pub mod oauth;

pub use api_key::*;
pub use jwt::*;
pub use middleware::*;
pub use oauth::*;
