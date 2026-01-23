//! API Server
//!
//! Axum-based HTTP server for the Maestro API.

use anyhow::Result;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use super::handlers::AppState;
use super::routes::build_router;
use crate::memory::MemoryService;

/// API Server configuration
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub db_path: Option<PathBuf>,
    pub enable_cors: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 18765,
            db_path: None,
            enable_cors: true,
        }
    }
}

/// Start the API server
pub async fn run_server(config: ServerConfig) -> Result<()> {
    // Initialize memory service
    let service = MemoryService::new(config.db_path)?;
    service.initialize()?;

    let state = Arc::new(AppState { service });

    // Build router
    let mut app = build_router(state);

    // Add CORS if enabled
    if config.enable_cors {
        app = app.layer(
            CorsLayer::new()
                .allow_origin([
                    "http://127.0.0.1:18765".parse().unwrap(),
                    "http://localhost:18765".parse().unwrap(),
                    "http://127.0.0.1:3000".parse().unwrap(), // Common dev port
                    "http://localhost:3000".parse().unwrap(),
                ])
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        );
    }

    // Add tracing
    app = app.layer(TraceLayer::new_for_http());

    // Bind and serve
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    info!("🚀 Maestro API server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
