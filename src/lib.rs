//! MCP Proxy - Intelligent bridge between MCP clients and diverse agents/endpoints
//!
//! This crate provides a proxy server that implements the Model Context Protocol (MCP)
//! and routes tool calls to various agents and endpoints without requiring full MCP
//! server implementations for each capability.

pub mod auth;
pub mod config;
pub mod discovery;
pub mod error;
pub mod grpc;
pub mod mcp;
pub mod metrics;
pub mod openai;
pub mod registry;
pub mod routing;
pub mod security;
pub mod services;
pub mod startup;
pub mod supervisor;
pub mod tls;
pub mod utils;
pub mod web;

pub use config::{Config, ExternalMcpServersConfig, HttpServiceConfig, SseServiceConfig, HttpAuthType, SseAuthType};
pub use error::{ProxyError, Result};
pub use utils::*;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default configuration file name
pub const DEFAULT_CONFIG_FILE: &str = "config.yaml";

/// Default server host
pub const DEFAULT_HOST: &str = "0.0.0.0";

/// Default server port
pub const DEFAULT_PORT: u16 = 3001;
