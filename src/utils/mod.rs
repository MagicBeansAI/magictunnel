//! Shared utilities for MagicTunnel
//!
//! This module contains common utility functions that are used across
//! multiple parts of the codebase to avoid duplication and ensure consistency.

pub mod tool_processing;
pub mod filename_parsing;
pub mod name_sanitizer;

pub use tool_processing::*;
pub use filename_parsing::*;
pub use name_sanitizer::{
    sanitize_capability_name,
    sanitize_tool_name,
    sanitize_and_ensure_unique,
    ensure_unique_capability_name,
};