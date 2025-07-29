//! Error handling module for MCP Proxy
//! 
//! This module provides comprehensive error types and utilities for the MCP proxy.

mod error;

// Re-export the main error types and utilities
pub use error::{ProxyError, Result};
