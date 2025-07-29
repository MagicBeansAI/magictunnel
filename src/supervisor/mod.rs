//! Supervisor Client Module
//! 
//! This module provides a client interface for communicating with the
//! MagicTunnel supervisor process for restart and process management operations.

pub mod client;
pub mod types;

pub use client::SupervisorClient;
pub use types::*;