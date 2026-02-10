//! Lattice Handlers
//!
//! Request handlers for lattice API endpoints.

use super::models::*;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

/// Application state for lattice handlers
#[derive(Clone)]
pub struct LatticeAppState {
    pub service: Arc<LatticeService>,
}

impl LatticeAppState {
    pub fn new(service: LatticeService) -> Self {
        Self {
            service: Arc::new(service),
        }
    }
}

// ============================================================================
// Service Stub (to be implemented with actual lattice logic)
// ============================================================================

/// Placeholder for the LatticeService implementation
/// This will be implemented with actual lattice analysis logic
pub struct LatticeService;

impl LatticeService {
    pub async fn list_files(&self, _limit: usize, _offset: usize) -> Result<Vec<String>, String> {
        Ok(vec![])
    }

    pub async fn get_file_metadata(&self, _path: &str) -> Result<Option<FileAnalysisMetadata>, String> {
        Ok(None)
    }

    pub async fn get_file_analysis(&self, _path: &str) -> Result<Option<LatticeAnalysisResult>, String> {
        Ok(None)
    }

    pub async fn get_layer_result(&self, _path: &str, _layer: LatticeLayer) -> Result<Option<serde_json::Value>, String> {
        Ok(None)
    }

    pub async fn analyze_file(&self, _path: &str, _layers: &[LatticeLayer]) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({"status": "pending"}))
    }

    pub async fn batch_analyze(&self, _paths: &[String], _layers: &[LatticeLayer]) -> Result<BatchAnalyzeResponse, String> {
        Ok(BatchAnalyzeResponse {
            completed: 0,
            failed: 0,
            skipped: 0,
            results: vec![],
        })
    }

    pub async fn search(&self, _query: &str, _layer: Option<LatticeLayer>, _limit: usize) -> Result<Vec<serde_json::Value>, String> {
        Ok(vec![])
    }

    pub async fn get_statistics(&self) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({}))
    }

    pub async fn clear_file_cache(&self, _path: &str) -> Result<(), String> {
        Ok(())
    }

    pub fn is_supported_extension(&self, _ext: &str) -> bool {
        false
    }
}

// ============================================================================
// Response Wrappers
// ============================================================================

#[derive(Debug, serde::Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: serde::Serialize> ApiResponse<T> {
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

// ============================================================================
// Handlers
// ============================================================================

/// Get lattice status and health
pub async fn lattice_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "lattice",
        "version": env!("CARGO_PKG_VERSION"),
        "layers": LatticeLayer::ALL.iter().map(|l| l.name()).collect::<Vec<_>>()
    }))
}

/// Get all analyzed files
pub async fn list_files(
    State(state): State<LatticeAppState>,
    Query(params): Query<LatticeSearchQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).min(1000);
    let offset = params.offset.unwrap_or(0);

    match state.service.list_files(limit, offset).await {
        Ok(files) => ApiResponse::ok(files),
        Err(e) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// Get analysis metadata for a specific file
pub async fn get_file_metadata(
    State(state): State<LatticeAppState>,
    Path(file_path): Path<String>,
) -> impl IntoResponse {
    // Decode URL-encoded path
    let decoded = percent_encoding::percent_decode_str(&file_path)
        .decode_utf8()
        .unwrap_or_default();

    match state.service.get_file_metadata(&decoded).await {
        Ok(Some(metadata)) => ApiResponse::ok(metadata),
        Ok(None) => ApiResponse::err(StatusCode::NOT_FOUND, "File not found in lattice"),
        Err(e) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// Get complete lattice analysis for a file
pub async fn get_file_analysis(
    State(state): State<LatticeAppState>,
    Path(file_path): Path<String>,
) -> impl IntoResponse {
    let decoded = percent_encoding::percent_decode_str(&file_path)
        .decode_utf8()
        .unwrap_or_default();

    match state.service.get_file_analysis(&decoded).await {
        Ok(Some(analysis)) => ApiResponse::ok(analysis),
        Ok(None) => ApiResponse::err(StatusCode::NOT_FOUND, "File analysis not found"),
        Err(e) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// Get a specific layer result for a file
pub async fn get_layer_result(
    State(state): State<LatticeAppState>,
    Path((file_path, layer)): Path<(String, String)>,
) -> impl IntoResponse {
    let decoded = percent_encoding::percent_decode_str(&file_path)
        .decode_utf8()
        .unwrap_or_default();

    let layer = match layer.parse::<LatticeLayer>() {
        Ok(l) => l,
        Err(_) => {
            return ApiResponse::err(
                StatusCode::BAD_REQUEST,
                &format!("Invalid layer: {layer}"),
            )
        }
    };

    match state.service.get_layer_result(&decoded, layer).await {
        Ok(Some(result)) => ApiResponse::ok(result),
        Ok(None) => ApiResponse::err(StatusCode::NOT_FOUND, "Layer result not found"),
        Err(e) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// Analyze a single file
pub async fn analyze_file(
    State(state): State<LatticeAppState>,
    Json(req): Json<AnalyzeRequest>,
) -> impl IntoResponse {
    // Validate file path exists
    if !std::path::Path::new(&req.file_path).exists() {
        return ApiResponse::err(StatusCode::NOT_FOUND, "File not found");
    }

    // Check file extension is supported
    let ext = std::path::Path::new(&req.file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    if !state.service.is_supported_extension(ext) {
        return ApiResponse::err(
            StatusCode::BAD_REQUEST,
            &format!("Unsupported file extension: {ext}"),
        );
    }

    // Use specified layers or default to all
    let layers = req.layers.unwrap_or_else(|| LatticeLayer::ALL.to_vec());

    match state.service.analyze_file(&req.file_path, &layers).await {
        Ok(result) => ApiResponse::ok(result),
        Err(e) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// Batch analyze multiple files
pub async fn batch_analyze(
    State(state): State<LatticeAppState>,
    Json(req): Json<BatchAnalyzeRequest>,
) -> impl IntoResponse {
    if req.file_paths.is_empty() {
        return ApiResponse::err(StatusCode::BAD_REQUEST, "No files provided");
    }

    if req.file_paths.len() > 100 {
        return ApiResponse::err(StatusCode::BAD_REQUEST, "Too many files (max 100)");
    }

    let layers = req.layers.unwrap_or_else(|| LatticeLayer::ALL.to_vec());

    match state.service.batch_analyze(&req.file_paths, &layers).await {
        Ok(result) => ApiResponse::ok(result),
        Err(e) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// Search the lattice
pub async fn search_lattice(
    State(state): State<LatticeAppState>,
    Query(params): Query<LatticeSearchQuery>,
) -> impl IntoResponse {
    let query = params.q.unwrap_or_default();
    let limit = params.limit.unwrap_or(50).min(1000);

    match state.service.search(&query, params.layer, limit).await {
        Ok(results) => ApiResponse::ok(results),
        Err(e) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// Get analysis statistics
pub async fn get_statistics(
    State(state): State<LatticeAppState>,
) -> impl IntoResponse {
    match state.service.get_statistics().await {
        Ok(stats) => ApiResponse::ok(stats),
        Err(e) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// Clear analysis cache for a file
pub async fn clear_file_cache(
    State(state): State<LatticeAppState>,
    Path(file_path): Path<String>,
) -> impl IntoResponse {
    let decoded = percent_encoding::percent_decode_str(&file_path)
        .decode_utf8()
        .unwrap_or_default();

    match state.service.clear_file_cache(&decoded).await {
        Ok(_) => ApiResponse::ok(serde_json::json!({"message": "Cache cleared"})),
        Err(e) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}
