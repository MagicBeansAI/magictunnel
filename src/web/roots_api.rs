//! MCP Roots API endpoints for filesystem/URI boundary management
//! 
//! Provides REST API for managing filesystem roots, URI boundaries, and access permissions.

use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::error::ProxyError;
use crate::mcp::roots::RootsService;
use crate::mcp::types::roots::{Root, RootType, RootFilter, RootsListRequest, RootPermission};

/// Root entry for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRootEntry {
    pub id: String,
    pub root_type: String, // "filesystem", "uri", "database", etc.
    pub path: String,
    pub name: Option<String>,
    pub permissions: Vec<String>, // ["read", "write", "execute", etc.]
    pub accessible: bool,
    pub discovered_at: String,
    pub manual: bool,
    pub metadata: HashMap<String, String>,
}

/// Request to add a manual root
#[derive(Debug, Deserialize)]
pub struct AddManualRootRequest {
    pub root_type: String,
    pub path: String,
    pub name: Option<String>,
    pub permissions: Vec<String>,
}

/// Request to update roots configuration
#[derive(Debug, Deserialize)]
pub struct UpdateRootsConfigRequest {
    pub blocked_patterns: Vec<String>,
    pub allowed_extensions: Vec<String>,
    pub blocked_extensions: Vec<String>,
    pub max_depth: Option<u32>,
    pub follow_symlinks: bool,
}

/// Roots service status
#[derive(Debug, Serialize)]
pub struct RootsServiceStatus {
    pub healthy: bool,
    pub total_roots: usize,
    pub accessible_roots: usize,
    pub manual_roots: usize,
    pub last_discovery: Option<String>,
    pub discovery_duration_ms: Option<u64>,
    pub cache_status: String,
}

/// Pagination parameters
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub cursor: Option<String>,
    pub limit: Option<usize>,
    pub filter_type: Option<String>,
    pub accessible_only: Option<bool>,
}

/// Paginated response
#[derive(Debug, Serialize)]
pub struct PaginatedRootsResponse {
    pub roots: Vec<Root>,
    pub next_cursor: Option<String>,
    pub total_count: Option<usize>,
    pub has_more: bool,
}

/// Roots API handler
pub struct RootsApiHandler {
    roots_service: Arc<RootsService>,
}

impl RootsApiHandler {
    pub fn new(roots_service: Arc<RootsService>) -> Self {
        Self { roots_service }
    }

    /// GET /api/roots/list - List discovered roots with pagination
    pub async fn list_roots(&self, params: web::Query<PaginationParams>) -> Result<HttpResponse> {
        debug!("Fetching roots list with pagination");
        
        let filter = RootFilter {
            types: params.filter_type.as_ref().map(|t| vec![parse_root_type(t)]),
            schemes: None,
            accessible_only: Some(params.accessible_only.unwrap_or(false)),
        };

        let request = RootsListRequest {
            cursor: params.cursor.clone(),
            limit: params.limit.map(|l| l as u32),
            filter: Some(filter),
        };

        match self.roots_service.handle_roots_list_request(request).await {
            Ok(response) => {
                let api_roots: Vec<Root> = response.roots;

                let has_more = response.next_cursor.is_some();
                let paginated_response = PaginatedRootsResponse {
                    roots: api_roots,
                    next_cursor: response.next_cursor,
                    total_count: response.total_count.map(|c| c as usize),
                    has_more,
                };

                Ok(HttpResponse::Ok().json(paginated_response))
            }
            Err(e) => {
                error!("Failed to list roots: {:?}", e);
                Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to list roots",
                    "details": format!("{:?}", e)
                })))
            }
        }
    }

    /// POST /api/roots/discover - Trigger manual root discovery
    pub async fn trigger_discovery(&self) -> Result<HttpResponse> {
        info!("Triggering manual roots discovery");
        
        match self.roots_service.trigger_discovery().await {
            Ok(discovery_result) => {
                let discovered_count = discovery_result.get("discovered_count").and_then(|v| v.as_u64()).unwrap_or(0);
                let duration_ms = discovery_result.get("discovery_duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);
                
                info!("Manual discovery completed: {} roots found", discovered_count);
                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "message": "Discovery completed successfully",
                    "total_discovered": discovered_count,
                    "new_roots": discovered_count, // For simplicity, treat all as new
                    "updated_roots": 0,
                    "duration_ms": duration_ms
                })))
            }
            Err(e) => {
                error!("Manual discovery failed: {:?}", e);
                Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Discovery failed",
                    "details": format!("{:?}", e)
                })))
            }
        }
    }

    /// GET /api/roots/status - Get service health and statistics
    pub async fn get_status(&self) -> Result<HttpResponse> {
        debug!("Fetching roots service status");
        
        let status = self.roots_service.get_service_status().await;
        
        let api_status = RootsServiceStatus {
            healthy: status.get("healthy").and_then(|v| v.as_bool()).unwrap_or(false),
            total_roots: status.get("total_roots").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
            accessible_roots: status.get("accessible_roots").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
            manual_roots: status.get("manual_roots").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
            last_discovery: status.get("last_discovery").and_then(|v| v.as_str()).map(|s| s.to_string()),
            discovery_duration_ms: status.get("discovery_duration_ms").and_then(|v| v.as_u64()),
            cache_status: status.get("cache_status").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
        };

        Ok(HttpResponse::Ok().json(api_status))
    }

    /// PUT /api/roots/config - Update security configuration
    pub async fn update_config(&self, request: web::Json<UpdateRootsConfigRequest>) -> Result<HttpResponse> {
        info!("Updating roots security configuration");
        
        // Convert to JSON for config update
        let config_update = serde_json::json!({
            "blocked_patterns": request.blocked_patterns,
            "allowed_extensions": request.allowed_extensions,
            "blocked_extensions": request.blocked_extensions,
            "max_depth": request.max_depth,
            "follow_symlinks": request.follow_symlinks
        });

        match self.roots_service.update_config(config_update).await {
            Ok(_) => {
                info!("Roots configuration updated successfully");
                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "message": "Configuration updated successfully"
                })))
            }
            Err(e) => {
                error!("Failed to update roots configuration: {:?}", e);
                Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to update configuration",
                    "details": format!("{:?}", e)
                })))
            }
        }
    }

    /// POST /api/roots/manual - Add manual root entry
    pub async fn add_manual_root(&self, request: web::Json<AddManualRootRequest>) -> Result<HttpResponse> {
        info!("Adding manual root: {} ({})", request.path, request.root_type);
        
        let root_type = parse_root_type(&request.root_type);
        
        let permissions: Vec<RootPermission> = request.permissions.iter()
            .map(|p| parse_permission(p))
            .collect();
        
        // Create a unique ID for the manual root
        let root_id = format!("manual_{}", Uuid::new_v4().to_string().replace('-', "")[..8].to_string());
        
        let manual_root = Root {
            id: root_id.clone(),
            root_type,
            uri: request.path.clone(),
            name: request.name.clone(),
            description: Some("Manually added root".to_string()),
            accessible: true, // Assume accessible for manual roots
            permissions: Some(permissions),
            metadata: None,
            tags: Some(vec!["manual".to_string()]),
        };

        match self.roots_service.add_manual_root(manual_root).await {
            Ok(()) => {
                info!("Manual root added successfully with ID: {}", root_id);
                Ok(HttpResponse::Created().json(serde_json::json!({
                    "message": "Manual root added successfully",
                    "root_id": root_id
                })))
            }
            Err(e) => {
                error!("Failed to add manual root: {:?}", e);
                Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to add manual root",
                    "details": format!("{:?}", e)
                })))
            }
        }
    }

    /// DELETE /api/roots/manual/{id} - Remove manual root
    pub async fn remove_manual_root(&self, path: web::Path<String>) -> Result<HttpResponse> {
        let root_id = path.into_inner();
        info!("Removing manual root: {}", root_id);
        
        match self.roots_service.remove_manual_root(&root_id).await {
            Ok(()) => {
                info!("Manual root removed successfully: {}", root_id);
                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "message": "Manual root removed successfully"
                })))
            }
            Err(e) => {
                error!("Failed to remove manual root: {:?}", e);
                Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to remove manual root",
                    "details": format!("{:?}", e)
                })))
            }
        }
    }

}

/// Configure roots API routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/roots")
            .route("/list", web::get().to(list_roots_handler))
            .route("/discover", web::post().to(trigger_discovery_handler))
            .route("/status", web::get().to(get_status_handler))
            .route("/config", web::put().to(update_config_handler))
            .route("/manual", web::post().to(add_manual_root_handler))
            .route("/manual/{id}", web::delete().to(remove_manual_root_handler)),
    );
}

// Handler functions for dependency injection
async fn list_roots_handler(
    handler: web::Data<RootsApiHandler>,
    params: web::Query<PaginationParams>,
) -> Result<HttpResponse> {
    handler.list_roots(params).await
}

async fn trigger_discovery_handler(
    handler: web::Data<RootsApiHandler>,
) -> Result<HttpResponse> {
    handler.trigger_discovery().await
}

async fn get_status_handler(
    handler: web::Data<RootsApiHandler>,
) -> Result<HttpResponse> {
    handler.get_status().await
}

async fn update_config_handler(
    handler: web::Data<RootsApiHandler>,
    request: web::Json<UpdateRootsConfigRequest>,
) -> Result<HttpResponse> {
    handler.update_config(request).await
}

async fn add_manual_root_handler(
    handler: web::Data<RootsApiHandler>,
    request: web::Json<AddManualRootRequest>,
) -> Result<HttpResponse> {
    handler.add_manual_root(request).await
}

async fn remove_manual_root_handler(
    handler: web::Data<RootsApiHandler>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    handler.remove_manual_root(path).await
}

/// Helper function to parse root type string to RootType enum
fn parse_root_type(type_str: &str) -> RootType {
    match type_str {
        "filesystem" => RootType::Filesystem,
        "uri" => RootType::Uri,
        "database" => RootType::Database,
        "api" => RootType::Api,
        "cloud_storage" => RootType::CloudStorage,
        "container" => RootType::Container,
        "network_share" => RootType::NetworkShare,
        other => RootType::Custom(other.to_string()),
    }
}

/// Helper function to parse permission string to RootPermission enum
fn parse_permission(perm_str: &str) -> RootPermission {
    match perm_str {
        "read" => RootPermission::Read,
        "write" => RootPermission::Write,
        "execute" => RootPermission::Execute,
        "create" => RootPermission::Create,
        "delete" => RootPermission::Delete,
        "list" => RootPermission::List,
        "modify" => RootPermission::Modify,
        "admin" => RootPermission::Admin,
        _ => RootPermission::Read, // Default fallback
    }
}