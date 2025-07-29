//! Routing module for directing tool calls to appropriate agents/endpoints

pub mod agent_router;
pub mod conflict_resolution;
pub mod enhanced_router;

pub mod middleware;
pub mod retry;
pub mod timeout;
pub mod router;
pub mod substitution;
pub mod types;

pub use agent_router::{AgentRouter, DefaultAgentRouter};
pub use conflict_resolution::{CapabilitySource, ConflictInfo, ConflictResolver, ConflictResolutionConfig, ConflictSource};
pub use enhanced_router::{EnhancedAgentRouter, EnhancedRouterBuilder};
// Legacy hybrid routing removed - use external_mcp instead
pub use middleware::{LoggingMiddleware, MetricsMiddleware, MiddlewareChain, MiddlewareContext, RouterMiddleware};
pub use router::Router;
pub use substitution::*;
pub use types::*;
