//! API Handlers
//!
//! Request handlers for all API endpoints.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::memory::{MemoryCategory, MemoryService};

/// Application state shared across handlers
pub struct AppState {
    pub service: MemoryService,
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> (StatusCode, Json<Self>) {
        (
            StatusCode::OK,
            Json(Self {
                success: true,
                data: Some(data),
                error: None,
            }),
        )
    }

    pub fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<Self>) {
        (
            status,
            Json(Self {
                success: false,
                data: None,
                error: Some(msg.to_string()),
            }),
        )
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub version: String,
    pub database: String,
    pub project_count: usize,
    pub memory_count: usize,
    pub track_count: usize,
}

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    pub paths: Vec<String>,
    pub max_depth: Option<usize>,
}

// ============================================================================
// Handlers
// ============================================================================

/// Health check endpoint
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Get system status
pub async fn status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let stats_result = tokio::task::spawn_blocking(move || state.service.stats()).await;

    match stats_result {
        Ok(Ok(stats)) => ApiResponse::ok(StatusResponse {
            version: env!("CARGO_PKG_VERSION").to_string(),
            database: "connected".to_string(),
            project_count: stats.project_count,
            memory_count: stats.memory_count,
            track_count: stats.track_count,
        }),
        Ok(Err(e)) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        Err(_) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, "Thread pool error"),
    }
}

/// List all projects
pub async fn list_projects(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || state.service.list_projects()).await;

    match result {
        Ok(Ok(projects)) => ApiResponse::ok(projects),
        Ok(Err(e)) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        Err(_) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, "Thread pool error"),
    }
}

/// Get a specific project
pub async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || state.service.get_project(id)).await;

    match result {
        Ok(Ok(Some(project))) => ApiResponse::ok(project),
        Ok(Ok(None)) => ApiResponse::err(StatusCode::NOT_FOUND, "Project not found"),
        Ok(Err(e)) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        Err(_) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, "Thread pool error"),
    }
}

/// List tracks for a project
pub async fn list_tracks(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<i64>,
) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || state.service.list_tracks(project_id)).await;

    match result {
        Ok(Ok(tracks)) => ApiResponse::ok(tracks),
        Ok(Err(e)) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        Err(_) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, "Thread pool error"),
    }
}

/// Search memories
pub async fn search_memories(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let query = params.q.unwrap_or_default();
    let limit = params.limit.unwrap_or(50).min(1000); // Cap limit

    let result =
        tokio::task::spawn_blocking(move || state.service.search_memories(&query, limit)).await;

    match result {
        Ok(Ok(memories)) => ApiResponse::ok(memories),
        Ok(Err(e)) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        Err(_) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, "Thread pool error"),
    }
}

/// Store a new memory
#[derive(Debug, Deserialize)]
pub struct StoreMemoryRequest {
    pub content: String,
    pub category: Option<String>,
}

pub async fn store_memory(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StoreMemoryRequest>,
) -> impl IntoResponse {
    // Basic length check
    if req.content.len() > 1_000_000 {
        return ApiResponse::err(StatusCode::PAYLOAD_TOO_LARGE, "Content too large");
    }

    let category = match req.category.as_deref() {
        Some("fact") => MemoryCategory::Fact,
        Some("pattern") => MemoryCategory::Pattern,
        Some("decision") => MemoryCategory::Decision,
        Some("observation") => MemoryCategory::Observation,
        Some("temporary") => MemoryCategory::Temporary,
        _ => MemoryCategory::Context,
    };

    let content = req.content.clone();
    let result =
        tokio::task::spawn_blocking(move || state.service.store_memory(&content, category)).await;

    match result {
        Ok(Ok(id)) => ApiResponse::ok(serde_json::json!({ "id": id })),
        Ok(Err(e)) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        Err(_) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, "Thread pool error"),
    }
}

/// Scan directories for projects
pub async fn scan_projects(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ScanRequest>,
) -> impl IntoResponse {
    // Validate paths - prevent absolute path probing outside allowed roots if needed
    // For now we just ensure they aren't completely crazy
    if req.paths.len() > 10 {
        return ApiResponse::err(StatusCode::BAD_REQUEST, "Too many paths to scan");
    }

    let paths: Vec<std::path::PathBuf> = req.paths.iter().map(|p| p.into()).collect();
    let depth = req.max_depth.unwrap_or(5).min(10); // Cap depth

    let result =
        tokio::task::spawn_blocking(move || state.service.scan_directories(&paths, depth)).await;

    match result {
        Ok(Ok(result)) => ApiResponse::ok(result),
        Ok(Err(e)) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        Err(_) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, "Thread pool error"),
    }
}
