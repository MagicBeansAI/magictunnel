//! Service management for MagicTunnel multi-mode architecture
//! 
//! This module provides conditional service loading based on runtime mode:
//! - Proxy mode: Core services only (MCP server, registry, optional smart discovery)
//! - Advanced mode: All services (core + enterprise security + tool enhancement)

pub mod proxy_services;
pub mod advanced_services;
pub mod service_loader;
pub mod service_container;
pub mod types;

pub use proxy_services::ProxyServices;
pub use advanced_services::AdvancedServices;
pub use service_loader::ServiceLoader;
pub use service_container::ServiceContainer;
pub use types::{ServiceLoadingSummary, ServiceStatus, ServiceState};