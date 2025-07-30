//! MCP Health Checker
//!
//! This module provides active health checking for External MCP services,
//! performing real MCP protocol health checks to determine service status.

use crate::mcp::external_process::ExternalMcpProcess;
use crate::mcp::metrics::{HealthCheckResult, HealthCheckType, HealthStatus};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

/// Configuration for health checking
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Timeout for individual health checks
    pub timeout_seconds: u64,
    /// How often to perform health checks
    pub check_interval_seconds: u64,
    /// Number of consecutive failures before marking as down
    pub max_consecutive_failures: u32,
    /// Minimum response time to consider healthy (ms)
    pub healthy_response_time_ms: u64,
    /// Maximum response time before marking as degraded (ms)  
    pub degraded_response_time_ms: u64,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 10,
            check_interval_seconds: 30,
            max_consecutive_failures: 3,
            healthy_response_time_ms: 1000,
            degraded_response_time_ms: 3000,
        }
    }
}

/// MCP Health Checker for External MCP services
pub struct McpHealthChecker {
    config: HealthCheckConfig,
}

impl McpHealthChecker {
    pub fn new(config: HealthCheckConfig) -> Self {
        Self { config }
    }

    /// Perform a comprehensive health check on an MCP process
    pub async fn perform_health_check(&self, process: &ExternalMcpProcess) -> HealthCheckResult {
        let start_time = Instant::now();
        let check_time = Utc::now();

        debug!("🏥 [HEALTH] Starting health check for service: {}", process.name);

        // Check if process is running first
        if !process.is_running().await {
            return HealthCheckResult {
                status: HealthStatus::Down,
                response_time_ms: None,
                error_details: Some("Process is not running".to_string()),
                last_checked: check_time,
                check_type: HealthCheckType::Active,
                consecutive_failures: 0, // Will be updated by caller
            };
        }

        // Perform MCP protocol health check
        match self.check_mcp_protocol_health(process).await {
            Ok(response_time_ms) => {
                let status = self.determine_status_from_response_time(response_time_ms);
                
                info!("✅ [HEALTH] Service '{}' health check passed: {:?} ({}ms)", 
                      process.name, status, response_time_ms);

                HealthCheckResult {
                    status,
                    response_time_ms: Some(response_time_ms),
                    error_details: None,
                    last_checked: check_time,
                    check_type: HealthCheckType::Active,
                    consecutive_failures: 0,
                }
            }
            Err(error) => {
                let elapsed_ms = start_time.elapsed().as_millis() as u64;
                
                warn!("❌ [HEALTH] Service '{}' health check failed: {} ({}ms)", 
                      process.name, error, elapsed_ms);

                HealthCheckResult {
                    status: HealthStatus::Unhealthy,
                    response_time_ms: Some(elapsed_ms),
                    error_details: Some(error),
                    last_checked: check_time,
                    check_type: HealthCheckType::Active,
                    consecutive_failures: 0, // Will be updated by caller
                }
            }
        }
    }

    /// Check MCP protocol health by sending a tools/list request
    async fn check_mcp_protocol_health(&self, process: &ExternalMcpProcess) -> Result<u64, String> {
        let start_time = Instant::now();

        // Create timeout for the health check
        let health_check_future = async {
            // Send tools/list request as a health check
            let params = json!({});
            
            debug!("🔍 [HEALTH] Sending tools/list health check to service: {}", process.name);
            
            match process.send_request("tools/list", Some(params)).await {
                Ok(response) => {
                    if let Some(error) = response.error {
                        return Err(format!("MCP error: {}", error.message));
                    }

                    if response.result.is_some() {
                        Ok(())
                    } else {
                        Err("No result in tools/list response".to_string())
                    }
                }
                Err(e) => {
                    Err(format!("Request failed: {}", e))
                }
            }
        };

        // Apply timeout to the health check
        match timeout(Duration::from_secs(self.config.timeout_seconds), health_check_future).await {
            Ok(Ok(())) => {
                let elapsed_ms = start_time.elapsed().as_millis() as u64;
                debug!("✅ [HEALTH] MCP protocol health check successful for '{}': {}ms", 
                       process.name, elapsed_ms);
                Ok(elapsed_ms)
            }
            Ok(Err(e)) => {
                Err(format!("Health check error: {}", e))
            }
            Err(_) => {
                Err(format!("Health check timed out after {}s", self.config.timeout_seconds))
            }
        }
    }

    /// Determine health status based on response time
    fn determine_status_from_response_time(&self, response_time_ms: u64) -> HealthStatus {
        if response_time_ms <= self.config.healthy_response_time_ms {
            HealthStatus::Healthy
        } else if response_time_ms <= self.config.degraded_response_time_ms {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        }
    }

    /// Perform a lightweight ping check (alternative to full protocol check)
    pub async fn perform_ping_check(&self, process: &ExternalMcpProcess) -> HealthCheckResult {
        let start_time = Instant::now();
        let check_time = Utc::now();

        debug!("🏓 [HEALTH] Starting ping check for service: {}", process.name);

        // Check if process is running
        if !process.is_running().await {
            return HealthCheckResult {
                status: HealthStatus::Down,
                response_time_ms: None,
                error_details: Some("Process is not running".to_string()),
                last_checked: check_time,
                check_type: HealthCheckType::Active,
                consecutive_failures: 0,
            };
        }

        // Try to send a ping request (if the MCP server supports it)
        // Otherwise fall back to tools/list
        match self.send_ping_request(process).await {
            Ok(response_time_ms) => {
                let status = self.determine_status_from_response_time(response_time_ms);
                
                debug!("🏓 [HEALTH] Service '{}' ping successful: {:?} ({}ms)", 
                       process.name, status, response_time_ms);

                HealthCheckResult {
                    status,
                    response_time_ms: Some(response_time_ms),
                    error_details: None,
                    last_checked: check_time,
                    check_type: HealthCheckType::Active,
                    consecutive_failures: 0,
                }
            }
            Err(error) => {
                let elapsed_ms = start_time.elapsed().as_millis() as u64;
                
                debug!("🏓 [HEALTH] Service '{}' ping failed: {} ({}ms)", 
                       process.name, error, elapsed_ms);

                HealthCheckResult {
                    status: HealthStatus::Unhealthy,
                    response_time_ms: Some(elapsed_ms),
                    error_details: Some(error),
                    last_checked: check_time,
                    check_type: HealthCheckType::Active,
                    consecutive_failures: 0,
                }
            }
        }
    }

    /// Send ping request (falls back to tools/list if ping not supported)
    async fn send_ping_request(&self, process: &ExternalMcpProcess) -> Result<u64, String> {
        let start_time = Instant::now();

        let ping_future = async {
            // Try ping first
            let ping_result = process.send_request("ping", Some(json!({}))).await;
            
            match ping_result {
                Ok(response) if response.error.is_none() => {
                    debug!("🏓 [HEALTH] Ping request successful for service: {}", process.name);
                    Ok(())
                }
                _ => {
                    // Fallback to tools/list if ping is not supported
                    debug!("🔄 [HEALTH] Ping not supported, falling back to tools/list for service: {}", process.name);
                    
                    match process.send_request("tools/list", Some(json!({}))).await {
                        Ok(response) => {
                            if response.error.is_some() {
                                Err("tools/list returned error".to_string())
                            } else {
                                Ok(())
                            }
                        }
                        Err(e) => Err(format!("tools/list failed: {}", e))
                    }
                }
            }
        };

        match timeout(Duration::from_secs(self.config.timeout_seconds), ping_future).await {
            Ok(Ok(())) => {
                let elapsed_ms = start_time.elapsed().as_millis() as u64;
                Ok(elapsed_ms)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(format!("Ping timed out after {}s", self.config.timeout_seconds))
        }
    }

    /// Perform a synthetic transaction test
    pub async fn perform_synthetic_check(&self, process: &ExternalMcpProcess) -> HealthCheckResult {
        let start_time = Instant::now();
        let check_time = Utc::now();

        debug!("🧪 [HEALTH] Starting synthetic check for service: {}", process.name);

        // Check if process is running
        if !process.is_running().await {
            return HealthCheckResult {
                status: HealthStatus::Down,
                response_time_ms: None,
                error_details: Some("Process is not running".to_string()),
                last_checked: check_time,
                check_type: HealthCheckType::Synthetic,
                consecutive_failures: 0,
            };
        }

        // Perform synthetic transaction: tools/list followed by a synthetic tools/call
        match self.perform_synthetic_transaction(process).await {
            Ok(response_time_ms) => {
                let status = self.determine_status_from_response_time(response_time_ms);
                
                info!("🧪 [HEALTH] Service '{}' synthetic check passed: {:?} ({}ms)", 
                      process.name, status, response_time_ms);

                HealthCheckResult {
                    status,
                    response_time_ms: Some(response_time_ms),
                    error_details: None,
                    last_checked: check_time,
                    check_type: HealthCheckType::Synthetic,
                    consecutive_failures: 0,
                }
            }
            Err(error) => {
                let elapsed_ms = start_time.elapsed().as_millis() as u64;
                
                warn!("🧪 [HEALTH] Service '{}' synthetic check failed: {} ({}ms)", 
                      process.name, error, elapsed_ms);

                HealthCheckResult {
                    status: HealthStatus::Unhealthy,
                    response_time_ms: Some(elapsed_ms),
                    error_details: Some(error),
                    last_checked: check_time,
                    check_type: HealthCheckType::Synthetic,
                    consecutive_failures: 0,
                }
            }
        }
    }

    /// Perform a synthetic transaction (list tools, then optionally call a safe tool)
    async fn perform_synthetic_transaction(&self, process: &ExternalMcpProcess) -> Result<u64, String> {
        let start_time = Instant::now();

        let synthetic_future = async {
            // Step 1: List available tools
            debug!("🧪 [HEALTH] Step 1: Listing tools for service: {}", process.name);
            
            let tools_response = process.send_request("tools/list", Some(json!({}))).await
                .map_err(|e| format!("Failed to list tools: {}", e))?;

            if let Some(error) = tools_response.error {
                return Err(format!("tools/list error: {}", error.message));
            }

            let tools_result = tools_response.result
                .ok_or_else(|| "No result in tools/list response".to_string())?;

            debug!("🧪 [HEALTH] Step 1 successful, tools listed for service: {}", process.name);

            // Step 2: For now, just validate that we got a proper tools response
            // In the future, we could try calling a safe tool if one exists
            if let Ok(tools_list) = serde_json::from_value::<crate::mcp::types::ToolListResponse>(tools_result) {
                debug!("🧪 [HEALTH] Synthetic transaction successful for '{}': {} tools available", 
                       process.name, tools_list.tools.len());
                Ok(())
            } else {
                Err("Invalid tools/list response format".to_string())
            }
        };

        match timeout(Duration::from_secs(self.config.timeout_seconds), synthetic_future).await {
            Ok(Ok(())) => {
                let elapsed_ms = start_time.elapsed().as_millis() as u64;
                Ok(elapsed_ms)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(format!("Synthetic check timed out after {}s", self.config.timeout_seconds))
        }
    }

    /// Get health check configuration
    pub fn config(&self) -> &HealthCheckConfig {
        &self.config
    }

    /// Update health check configuration
    pub fn update_config(&mut self, config: HealthCheckConfig) {
        self.config = config;
        info!("🔧 [HEALTH] Updated health check configuration: timeout={}s, interval={}s", 
              self.config.timeout_seconds, self.config.check_interval_seconds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_determination() {
        let config = HealthCheckConfig::default();
        let checker = McpHealthChecker::new(config);

        // Test healthy response time
        assert_eq!(checker.determine_status_from_response_time(500), HealthStatus::Healthy);
        
        // Test degraded response time
        assert_eq!(checker.determine_status_from_response_time(2000), HealthStatus::Degraded);
        
        // Test unhealthy response time
        assert_eq!(checker.determine_status_from_response_time(5000), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_health_check_config_defaults() {
        let config = HealthCheckConfig::default();
        
        assert_eq!(config.timeout_seconds, 10);
        assert_eq!(config.check_interval_seconds, 30);
        assert_eq!(config.max_consecutive_failures, 3);
        assert_eq!(config.healthy_response_time_ms, 1000);
        assert_eq!(config.degraded_response_time_ms, 3000);
    }
}