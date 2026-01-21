//! API Routes
//!
//! Route definitions for the Maestro API.

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use super::handlers::{self, AppState};

/// Build the API router
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health and status
        .route("/api/health", get(handlers::health))
        .route("/api/v1/status", get(handlers::status))
        // Projects
        .route("/api/v1/projects", get(handlers::list_projects))
        .route("/api/v1/projects/:id", get(handlers::get_project))
        .route("/api/v1/projects/:id/tracks", get(handlers::list_tracks))
        // Memories
        .route("/api/v1/memories", get(handlers::search_memories))
        .route("/api/v1/memories", post(handlers::store_memory))
        // Scanning
        .route("/api/v1/scan", post(handlers::scan_projects))
        // State
        .with_state(state)
}
