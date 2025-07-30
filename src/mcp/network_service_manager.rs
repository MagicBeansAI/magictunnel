//! Network MCP Service Manager
//! 
//! This module manages network-based MCP services (HTTP, SSE, WebSocket) and provides
//! capability discovery, tool execution, and lifecycle management for remote endpoints.

use crate::config::{ExternalMcpServersConfig, HttpServiceConfig, SseServiceConfig};
use crate::error::{ProxyError, Result};
use crate::mcp::clients::{HttpMcpClient, SseMcpClient};
use crate::mcp::types::{Tool, McpRequest, McpResponse};
use crate::mcp::metrics::{McpMetricsCollector, McpHealthThresholds, HealthStatus};
use crate::mcp::health_checker::{McpHealthChecker, HealthCheckConfig};
use crate::registry::types::{CapabilityFile, ToolDefinition, RoutingConfig};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// Network MCP Service Types
#[derive(Debug, Clone)]
pub enum NetworkMcpService {
    Http(HttpMcpClient),
    Sse(SseMcpClient),
    // WebSocket(WebSocketMcpClient), // Future
}

impl NetworkMcpService {
    /// Get the service ID
    pub fn service_id(&self) -> &str {
        match self {
            NetworkMcpService::Http(client) => client.service_id(),
            NetworkMcpService::Sse(client) => client.service_id(),
        }
    }

    /// List tools from the service
    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        match self {
            NetworkMcpService::Http(client) => client.list_tools().await,
            NetworkMcpService::Sse(client) => client.list_tools().await,
        }
    }

    /// Call a tool on the service
    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        match self {
            NetworkMcpService::Http(client) => client.call_tool(tool_name, arguments).await,
            NetworkMcpService::Sse(client) => client.call_tool(tool_name, arguments).await,
        }
    }

    /// Perform health check on the service
    pub async fn health_check(&self) -> Result<bool> {
        match self {
            NetworkMcpService::Http(client) => client.health_check().await,
            NetworkMcpService::Sse(client) => client.health_check().await,
        }
    }

    /// Clear service cache
    pub async fn clear_cache(&self) {
        match self {
            NetworkMcpService::Http(client) => client.clear_cache().await,
            NetworkMcpService::Sse(client) => client.clear_cache().await,
        }
    }
}

/// Manages network-based MCP services
pub struct NetworkMcpServiceManager {
    /// Configuration for network MCP services
    config: ExternalMcpServersConfig,
    /// Active network MCP services
    services: Arc<RwLock<HashMap<String, NetworkMcpService>>>,
    /// Discovered capabilities from all services
    capabilities: Arc<RwLock<HashMap<String, Vec<Tool>>>>,
    /// Metrics collector for observability
    metrics_collector: Arc<McpMetricsCollector>,
    /// Health checker for active monitoring
    health_checker: Arc<McpHealthChecker>,
    /// Output directory for capability files
    capabilities_output_dir: String,
}

impl NetworkMcpServiceManager {
    /// Create a new Network MCP Service Manager
    pub fn new(config: ExternalMcpServersConfig, capabilities_output_dir: String) -> Self {
        // Initialize metrics collector with default thresholds
        let metrics_collector = Arc::new(McpMetricsCollector::new(McpHealthThresholds::default()));
        
        // Initialize health checker with default configuration
        let health_checker = Arc::new(McpHealthChecker::new(HealthCheckConfig::default()));

        Self {
            config,
            services: Arc::new(RwLock::new(HashMap::new())),
            capabilities: Arc::new(RwLock::new(HashMap::new())),
            metrics_collector,
            health_checker,
            capabilities_output_dir,
        }
    }

    /// Initialize all configured network services
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing Network MCP Service Manager");

        // Initialize HTTP services
        if let Some(http_services) = &self.config.http_services {
            for (service_id, service_config) in http_services {
                if service_config.enabled {
                    self.initialize_http_service(service_id, service_config).await?;
                }
            }
        }

        // Initialize SSE services
        if let Some(sse_services) = &self.config.sse_services {
            for (service_id, service_config) in sse_services {
                if service_config.enabled {
                    self.initialize_sse_service(service_id, service_config).await?;
                }
            }
        }

        // Start background tasks
        self.start_background_tasks().await;

        info!("Network MCP Service Manager initialized successfully");
        Ok(())
    }

    /// Initialize an HTTP MCP service
    async fn initialize_http_service(&self, service_id: &str, config: &HttpServiceConfig) -> Result<()> {
        info!("Initializing HTTP MCP service: {}", service_id);

        // Convert config to client config
        let client_config = config.into();
        
        // Create HTTP client
        let client = HttpMcpClient::new(client_config, service_id.to_string())
            .map_err(|e| ProxyError::config(format!("Failed to create HTTP client for {}: {}", service_id, e)))?;

        // Test connection and discover capabilities
        match client.list_tools().await {
            Ok(tools) => {
                info!("HTTP service {} connected successfully with {} tools", service_id, tools.len());
                
                // Store capabilities
                {
                    let mut capabilities = self.capabilities.write().await;
                    capabilities.insert(service_id.to_string(), tools.clone());
                }

                // Generate capability file
                self.generate_capability_file(service_id, &tools, "http").await?;

                // Store service
                {
                    let mut services = self.services.write().await;
                    services.insert(service_id.to_string(), NetworkMcpService::Http(client));
                }

                // Record metrics
                self.metrics_collector.update_health_status(service_id, HealthStatus::Healthy, None).await;
            }
            Err(e) => {
                warn!("Failed to connect to HTTP service {}: {}", service_id, e);
                self.metrics_collector.update_health_status(service_id, HealthStatus::Unhealthy, None).await;
                return Err(e);
            }
        }

        Ok(())
    }

    /// Initialize an SSE MCP service
    async fn initialize_sse_service(&self, service_id: &str, config: &SseServiceConfig) -> Result<()> {
        info!("Initializing SSE MCP service: {}", service_id);

        // Convert config to client config
        let client_config = config.into();
        
        // Create SSE client
        let client = SseMcpClient::new(client_config, service_id.to_string())
            .map_err(|e| ProxyError::config(format!("Failed to create SSE client for {}: {}", service_id, e)))?;

        // Connect and discover capabilities
        match client.connect().await {
            Ok(_) => {
                match client.list_tools().await {
                    Ok(tools) => {
                        info!("SSE service {} connected successfully with {} tools", service_id, tools.len());
                        
                        // Store capabilities
                        {
                            let mut capabilities = self.capabilities.write().await;
                            capabilities.insert(service_id.to_string(), tools.clone());
                        }

                        // Generate capability file
                        self.generate_capability_file(service_id, &tools, "sse").await?;

                        // Store service
                        {
                            let mut services = self.services.write().await;
                            services.insert(service_id.to_string(), NetworkMcpService::Sse(client));
                        }

                        // Record metrics
                        self.metrics_collector.update_health_status(service_id, HealthStatus::Healthy, None).await;
                    }
                    Err(e) => {
                        warn!("Failed to list tools from SSE service {}: {}", service_id, e);
                        self.metrics_collector.update_health_status(service_id, HealthStatus::Degraded, None).await;
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to connect to SSE service {}: {}", service_id, e);
                self.metrics_collector.update_health_status(service_id, HealthStatus::Unhealthy, None).await;
                return Err(e);
            }
        }

        Ok(())
    }

    /// Generate capability file for a service
    async fn generate_capability_file(&self, service_id: &str, tools: &[Tool], service_type: &str) -> Result<()> {
        let output_dir = Path::new(&self.capabilities_output_dir);
        
        // Create output directory if it doesn't exist
        if !output_dir.exists() {
            tokio::fs::create_dir_all(output_dir).await
                .map_err(|e| ProxyError::config(format!("Failed to create capabilities directory: {}", e)))?;
        }

        // Convert tools to capability format
        let tool_definitions: Vec<ToolDefinition> = tools.iter().map(|tool| {
            let routing_config = RoutingConfig::new(
                "external_mcp".to_string(),
                serde_json::json!({
                    "service_id": format!("network-{}-{}", service_type, service_id),
                    "priority": 50
                })
            );
            
            ToolDefinition {
                name: tool.name.clone(),
                description: tool.description.clone().unwrap_or_default(),
                input_schema: tool.input_schema.clone(),
                routing: routing_config,
                annotations: None,
                hidden: false,
                enabled: true,
            }
        }).collect();

        let metadata = crate::registry::types::FileMetadata::with_name(
            format!("network-{}-{}", service_type, service_id)
        )
        .description(format!("Network MCP {} service: {}", service_type.to_uppercase(), service_id))
        .version("1.0".to_string());

        let capability_file = CapabilityFile::with_metadata(metadata, tool_definitions)
            .map_err(|e| ProxyError::config(format!("Failed to create capability file: {}", e)))?;

        // Write capability file
        let file_path = output_dir.join(format!("network-{}-{}.yaml", service_type, service_id));
        let yaml_content = serde_yaml::to_string(&capability_file)
            .map_err(|e| ProxyError::config(format!("Failed to serialize capability file: {}", e)))?;

        tokio::fs::write(&file_path, yaml_content).await
            .map_err(|e| ProxyError::config(format!("Failed to write capability file: {}", e)))?;

        debug!("Generated capability file: {:?}", file_path);
        Ok(())
    }

    /// Start background monitoring tasks
    async fn start_background_tasks(&self) {
        let services = Arc::clone(&self.services);
        let capabilities = Arc::clone(&self.capabilities);
        let metrics_collector = Arc::clone(&self.metrics_collector);

        // Health check task
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Check every minute
            
            loop {
                interval.tick().await;
                
                let services_guard = services.read().await;
                for (service_id, service) in services_guard.iter() {
                    match service.health_check().await {
                        Ok(is_healthy) => {
                            let status = if is_healthy { HealthStatus::Healthy } else { HealthStatus::Degraded };
                            metrics_collector.update_health_status(service_id, status, None).await;
                        }
                        Err(e) => {
                            warn!("Health check failed for network service {}: {}", service_id, e);
                            metrics_collector.update_health_status(service_id, HealthStatus::Unhealthy, None).await;
                        }
                    }
                }
            }
        });

        info!("Started network MCP service background monitoring tasks");
    }

    /// Get all available tools from all services
    pub async fn get_all_tools(&self) -> HashMap<String, Vec<Tool>> {
        let capabilities = self.capabilities.read().await;
        capabilities.clone()
    }

    /// Get tools from a specific service
    pub async fn get_service_tools(&self, service_id: &str) -> Option<Vec<Tool>> {
        let capabilities = self.capabilities.read().await;
        capabilities.get(service_id).cloned()
    }

    /// Call a tool on a specific service
    pub async fn call_tool(&self, service_id: &str, tool_name: &str, arguments: Value) -> Result<Value> {
        let services = self.services.read().await;
        
        if let Some(service) = services.get(service_id) {
            let start_time = Instant::now();
            let result = service.call_tool(tool_name, arguments).await;
            let duration = start_time.elapsed();

            // Record metrics
            match &result {
                Ok(_) => {
                    self.metrics_collector.record_request_success(service_id, duration.as_millis() as f64, tool_name).await;
                }
                Err(e) => {
                    self.metrics_collector.record_request_error(service_id, "tool_call_failed", tool_name).await;
                    warn!("Tool call failed for {} on {}: {}", tool_name, service_id, e);
                }
            }

            result
        } else {
            Err(ProxyError::routing(format!("Network service not found: {}", service_id)))
        }
    }

    /// Get health status of all services
    pub async fn get_health_status(&self) -> HashMap<String, HealthStatus> {
        let all_metrics = self.metrics_collector.get_all_metrics().await;
        all_metrics.into_iter()
            .map(|(service_id, metrics)| (service_id, metrics.current_status))
            .collect()
    }

    /// Get active service IDs
    pub async fn get_active_services(&self) -> Vec<String> {
        let services = self.services.read().await;
        services.keys().cloned().collect()
    }

    /// Refresh capabilities for all services
    pub async fn refresh_capabilities(&self) -> Result<()> {
        info!("Refreshing capabilities for all network services");

        let services = self.services.read().await;
        let mut updated_capabilities = HashMap::new();

        for (service_id, service) in services.iter() {
            match service.list_tools().await {
                Ok(tools) => {
                    info!("Refreshed {} tools for service {}", tools.len(), service_id);
                    
                    // Determine service type for capability file generation
                    let service_type = match service {
                        NetworkMcpService::Http(_) => "http",
                        NetworkMcpService::Sse(_) => "sse",
                    };

                    // Generate updated capability file
                    if let Err(e) = self.generate_capability_file(service_id, &tools, service_type).await {
                        warn!("Failed to generate capability file for {}: {}", service_id, e);
                    }

                    updated_capabilities.insert(service_id.clone(), tools);
                }
                Err(e) => {
                    warn!("Failed to refresh capabilities for service {}: {}", service_id, e);
                }
            }
        }

        // Update stored capabilities
        {
            let mut capabilities = self.capabilities.write().await;
            for (service_id, tools) in updated_capabilities {
                capabilities.insert(service_id, tools);
            }
        }

        info!("Capability refresh completed");
        Ok(())
    }

    /// Shutdown all network services
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Network MCP Service Manager");

        let mut services = self.services.write().await;
        
        // Disconnect SSE services gracefully
        for (service_id, service) in services.iter() {
            if let NetworkMcpService::Sse(client) = service {
                if let Err(e) = client.disconnect().await {
                    warn!("Failed to disconnect SSE service {}: {}", service_id, e);
                }
            }
        }

        services.clear();
        
        info!("Network MCP Service Manager shutdown completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{HttpAuthType, SseAuthType};

    #[test]
    fn test_network_service_manager_creation() {
        let config = ExternalMcpServersConfig::default();
        let manager = NetworkMcpServiceManager::new(config, "./test-capabilities".to_string());
        
        // Basic smoke test - manager should be created successfully
        assert_eq!(manager.capabilities_output_dir, "./test-capabilities");
    }

    #[tokio::test]
    async fn test_empty_services_initialization() {
        let config = ExternalMcpServersConfig::default();
        let manager = NetworkMcpServiceManager::new(config, "./test-capabilities".to_string());
        
        // Should initialize successfully with no services
        let result = manager.initialize().await;
        assert!(result.is_ok());
        
        let active_services = manager.get_active_services().await;
        assert!(active_services.is_empty());
    }
}