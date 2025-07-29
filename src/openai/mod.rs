//! OpenAI API Compatibility Module
//!
//! This module provides OpenAI API compatibility for MagicTunnel, enabling:
//! - Custom GPT Actions integration via OpenAPI specs
//! - OpenAI API compatibility for function calling
//! - Broader ecosystem access for MCP tools

pub mod generator;
pub mod types;

pub use generator::*;
pub use types::*;