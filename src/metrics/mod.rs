//! Metrics Collection and Analysis Module
//!
//! This module provides comprehensive metrics collection for MCP services, smart discovery,
//! and individual tools. It enables real-time observability, performance tracking, and
//! analytics across the entire MagicTunnel system.

pub mod tool_metrics;

pub use tool_metrics::{
    ToolExecutionRecord, ToolExecutionResult, ToolMetrics, ToolMetricsCollector,
    ToolMetricsSummary, DiscoveryRanking,
};

// Re-export all public items at the crate level for easier access
pub use self::tool_metrics::*;

// Re-export MCP metrics from mcp module for convenience
pub use crate::mcp::metrics::{
    McpMetricsCollector, McpServiceMetrics, McpMetricsSummary, HealthStatus,
};