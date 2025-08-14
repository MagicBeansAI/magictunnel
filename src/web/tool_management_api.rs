//! Tool Management API endpoints for dynamic tool visibility and enable/disable
//!
//! Provides REST API endpoints for managing tool state without server restart

use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::services::tool_management::{ToolManagementService, UpdateToolStateRequest, BulkUpdateRequest};

/// Tool management API handler
pub struct ToolManagementApiHandler {
    tool_management: Arc<ToolManagementService>,
}

/// Quick action request for common operations
#[derive(Debug, Deserialize)]
pub struct QuickActionRequest {
    pub action: QuickAction,
}

/// Quick actions for bulk operations
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuickAction {
    HideAll,
    ShowAll,
    EnableAll,
    DisableAll,
}

/// Tool management statistics response
#[derive(Debug, Serialize)]
pub struct ToolStatsResponse {
    pub statistics: serde_json::Value,
    pub last_updated: String,
}

impl ToolManagementApiHandler {
    /// Create a new tool management API handler
    pub fn new(tool_management: Arc<ToolManagementService>) -> Self {
        Self { tool_management }
    }

    /// GET /api/tools/management/list - Get all tool states
    pub async fn list_tools(&self) -> Result<HttpResponse> {
        debug!("Fetching all tool states");
        
        match self.tool_management.get_all_tool_states().await {
            Ok(tools) => {
                info!("Retrieved {} tool states", tools.len());
                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "tools": tools,
                    "total_count": tools.len(),
                    "last_updated": chrono::Utc::now().to_rfc3339()
                })))
            }
            Err(e) => {
                error!("Failed to get tool states: {}", e);
                Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to retrieve tool states",
                    "details": e.to_string()
                })))
            }
        }
    }

    /// GET /api/tools/management/{name} - Get specific tool state
    pub async fn get_tool(&self, path: web::Path<String>) -> Result<HttpResponse> {
        let tool_name = path.into_inner();
        debug!("Fetching tool state for: {}", tool_name);
        
        match self.tool_management.get_tool_state(&tool_name).await {
            Ok(tool_state) => {
                Ok(HttpResponse::Ok().json(tool_state))
            }
            Err(e) => {
                warn!("Tool '{}' not found: {}", tool_name, e);
                Ok(HttpResponse::NotFound().json(serde_json::json!({
                    "error": "Tool not found",
                    "tool_name": tool_name,
                    "details": e.to_string()
                })))
            }
        }
    }

    /// PUT /api/tools/management/{name} - Update specific tool state
    pub async fn update_tool(
        &self, 
        path: web::Path<String>, 
        request: web::Json<UpdateToolStateRequest>
    ) -> Result<HttpResponse> {
        let tool_name = path.into_inner();
        info!("Updating tool '{}' with state: {:?}", tool_name, *request);
        
        match self.tool_management.update_tool_state(&tool_name, request.into_inner()).await {
            Ok(result) => {
                Ok(HttpResponse::Ok().json(result))
            }
            Err(e) => {
                error!("Failed to update tool '{}': {}", tool_name, e);
                Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to update tool",
                    "tool_name": tool_name,
                    "details": e.to_string()
                })))
            }
        }
    }

    /// PUT /api/tools/management/bulk - Bulk update multiple tools
    pub async fn bulk_update(&self, request: web::Json<BulkUpdateRequest>) -> Result<HttpResponse> {
        info!("Bulk updating {} tools", request.tool_names.len());
        
        if request.tool_names.is_empty() {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "No tools specified for bulk update"
            })));
        }

        match self.tool_management.bulk_update_tools(request.into_inner()).await {
            Ok(result) => {
                Ok(HttpResponse::Ok().json(result))
            }
            Err(e) => {
                error!("Failed to bulk update tools: {}", e);
                Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to bulk update tools",
                    "details": e.to_string()
                })))
            }
        }
    }

    /// POST /api/tools/management/quick-action - Perform quick actions (hide all, show all, etc.)
    pub async fn quick_action(&self, request: web::Json<QuickActionRequest>) -> Result<HttpResponse> {
        info!("Performing quick action: {:?}", request.action);
        
        let result = match request.action {
            QuickAction::HideAll => self.tool_management.hide_all_tools().await,
            QuickAction::ShowAll => self.tool_management.show_all_tools().await,
            QuickAction::EnableAll => self.tool_management.enable_all_tools().await,
            QuickAction::DisableAll => self.tool_management.disable_all_tools().await,
        };

        match result {
            Ok(management_result) => {
                Ok(HttpResponse::Ok().json(management_result))
            }
            Err(e) => {
                error!("Failed to perform quick action {:?}: {}", request.action, e);
                Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to perform quick action",
                    "action": format!("{:?}", request.action),
                    "details": e.to_string()
                })))
            }
        }
    }

    /// GET /api/tools/management/statistics - Get tool management statistics
    pub async fn get_statistics(&self) -> Result<HttpResponse> {
        debug!("Fetching tool management statistics");
        
        match self.tool_management.get_tool_statistics().await {
            Ok(stats) => {
                Ok(HttpResponse::Ok().json(ToolStatsResponse {
                    statistics: stats,
                    last_updated: chrono::Utc::now().to_rfc3339(),
                }))
            }
            Err(e) => {
                error!("Failed to get tool statistics: {}", e);
                Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to retrieve tool statistics",
                    "details": e.to_string()
                })))
            }
        }
    }

    /// POST /api/tools/management/refresh - Refresh tool cache
    pub async fn refresh_cache(&self) -> Result<HttpResponse> {
        info!("Refreshing tool management cache");
        
        // Re-initialize the service to refresh cache
        match self.tool_management.initialize().await {
            Ok(_) => {
                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "message": "Tool cache refreshed successfully",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })))
            }
            Err(e) => {
                error!("Failed to refresh tool cache: {}", e);
                Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to refresh tool cache",
                    "details": e.to_string()
                })))
            }
        }
    }
}

/// Configure tool management API routes with custom scope
pub fn configure_routes_with_scope(cfg: &mut web::ServiceConfig, scope_prefix: &str) {
    info!("🔧 Configuring tool management routes with scope {}/tools/management", scope_prefix);
    cfg.service(
        web::scope(&format!("{}/tools/management", scope_prefix))
            .route("/list", web::get().to(list_tools_handler))
            .route("/statistics", web::get().to(get_statistics_handler))
            .route("/refresh", web::post().to(refresh_cache_handler))
            .route("/quick-action", web::post().to(quick_action_handler))
            .route("/bulk", web::put().to(bulk_update_handler))
            .route("/{name}", web::get().to(get_tool_handler))
            .route("/{name}", web::put().to(update_tool_handler))
    );
    debug!("✅ Tool management routes configured: GET/PUT /{{name}}, PUT /bulk, POST /quick-action");
}

/// Configure tool management API routes (default scope)
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    configure_routes_with_scope(cfg, "");
}

// Route handlers

pub async fn list_tools_handler(
    handler: web::Data<ToolManagementApiHandler>,
) -> Result<HttpResponse> {
    handler.list_tools().await
}

pub async fn get_tool_handler(
    handler: web::Data<ToolManagementApiHandler>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    handler.get_tool(path).await
}

pub async fn update_tool_handler(
    handler: web::Data<ToolManagementApiHandler>,
    path: web::Path<String>,
    request: web::Json<UpdateToolStateRequest>,
) -> Result<HttpResponse> {
    info!("🔧 Tool management update request for tool: {}", path.as_ref());
    handler.update_tool(path, request).await
}

pub async fn bulk_update_handler(
    handler: web::Data<ToolManagementApiHandler>,
    request: web::Json<BulkUpdateRequest>,
) -> Result<HttpResponse> {
    handler.bulk_update(request).await
}

pub async fn quick_action_handler(
    handler: web::Data<ToolManagementApiHandler>,
    request: web::Json<QuickActionRequest>,
) -> Result<HttpResponse> {
    handler.quick_action(request).await
}

pub async fn get_statistics_handler(
    handler: web::Data<ToolManagementApiHandler>,
) -> Result<HttpResponse> {
    handler.get_statistics().await
}

pub async fn refresh_cache_handler(
    handler: web::Data<ToolManagementApiHandler>,
) -> Result<HttpResponse> {
    handler.refresh_cache().await
}