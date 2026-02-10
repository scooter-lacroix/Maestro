//! Lattice Routes
//!
//! Route definitions for the lattice API.

use axum::{
    routing::{get, post, delete},
    Router,
};

use super::handlers::{self, LatticeAppState};

/// Build the lattice API router
pub fn build_router(state: LatticeAppState) -> Router {
    Router::new()
        // Health and info
        .route("/lattice/health", get(handlers::lattice_health))
        .route("/lattice/stats", get(handlers::get_statistics))
        // File listing and search
        .route("/lattice/files", get(handlers::list_files))
        .route("/lattice/search", get(handlers::search_lattice))
        // Individual file operations
        .route(
            "/lattice/files/:file_path/metadata",
            get(handlers::get_file_metadata),
        )
        .route(
            "/lattice/files/:file_path/analysis",
            get(handlers::get_file_analysis),
        )
        .route(
            "/lattice/files/:file_path/layers/:layer",
            get(handlers::get_layer_result),
        )
        .route(
            "/lattice/files/:file_path/cache",
            delete(handlers::clear_file_cache),
        )
        // Analysis operations
        .route("/lattice/analyze", post(handlers::analyze_file))
        .route("/lattice/batch", post(handlers::batch_analyze))
        .with_state(state)
}
