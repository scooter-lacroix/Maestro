//! Gateway server implementation
//!
//! Main Axum server with WebSocket, SSE, and HTTP routes.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::get,
    Router,
};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::MakeRequestUuid,
    trace::TraceLayer,
    ServiceBuilderExt,
};
use tracing::{info, warn};

use crate::routes::create_routes;
use crate::sse::{sse_handler, sse_heartbeat};
use crate::state::{GatewayConfig, GatewayState};
use crate::ws::ws_handler;

/// Run the gateway server
pub async fn run(config: GatewayConfig) -> anyhow::Result<()> {
    let state = Arc::new(GatewayState::with_config(config.clone()));

    run_with_state(config, state).await
}

/// Run the gateway server with existing state
pub async fn run_with_state(
    config: GatewayConfig,
    state: Arc<GatewayState>,
) -> anyhow::Result<()> {
    // Build the router
    let app = create_app(state.clone(), config.clone());

    // Bind to address
    let addr: SocketAddr = format!("{}:{}", config.bind_address, config.port).parse()?;
    let listener = TcpListener::bind(addr).await?;

    info!("Maestro Gateway listening on {}", addr);

    // Run the server
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

/// Create the Axum application
pub fn create_app(state: Arc<GatewayState>, config: GatewayConfig) -> Router {
    // Build CORS layer based on configuration
    let cors = if config.cors_allowed_origins.is_empty() {
        warn!("CORS: No allowed origins configured - denying all CORS requests");
        // Deny all CORS requests
        CorsLayer::new()
    } else if config.cors_allowed_origins.iter().any(|o| o == "*") {
        warn!("CORS: Permissive mode enabled - allowing all origins. NOT RECOMMENDED FOR PRODUCTION!");
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        // Restrictive CORS with specific origins
        let origins: Result<Vec<_>, _> = config
            .cors_allowed_origins
            .iter()
            .map(|s| s.parse())
            .collect();

        match origins {
            Ok(parsed_origins) => {
                info!(
                    "CORS: Restrictive mode enabled - allowing {} origins",
                    parsed_origins.len()
                );
                let methods: Result<Vec<_>, _> = config
                    .cors_allowed_methods
                    .iter()
                    .map(|s| s.parse())
                    .collect();

                let headers: Result<Vec<_>, _> = config
                    .cors_allowed_headers
                    .iter()
                    .map(|s| s.parse())
                    .collect();

                match (methods, headers) {
                    (Ok(methods), Ok(headers)) => {
                        CorsLayer::new()
                            .allow_origin(parsed_origins)
                            .allow_methods(methods)
                            .allow_headers(headers)
                    }
                    _ => {
                        warn!("CORS: Invalid methods or headers configuration, using restrictive defaults");
                        CorsLayer::new()
                    }
                }
            }
            Err(_) => {
                warn!("CORS: Invalid origin configuration, using restrictive defaults");
                CorsLayer::new()
            }
        }
    };

    // Build middleware stack
    let middleware = ServiceBuilder::new()
        // Request ID
        .set_x_request_id(MakeRequestUuid)
        // Trace layer for logging
        .layer(TraceLayer::new_for_http())
        // CORS (configurable)
        .layer(cors)
        // Propagate request ID
        .propagate_x_request_id();

    // Build the router with state
    Router::new()
        // WebSocket endpoint
        .route("/ws", get(ws_handler))

        // SSE endpoints
        .route("/events", get(sse_handler))
        .route("/events/{types}", get(crate::sse::sse_events_handler))
        .route("/heartbeat", get(sse_heartbeat))

        // Merge HTTP API routes
        .merge(create_routes())

        // Apply middleware
        .layer(middleware)

        // State
        .with_state(state)
}

/// Shutdown signal handler
pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_app() {
        let config = GatewayConfig::default();
        let state = Arc::new(GatewayState::with_config(config.clone()));
        let _app = create_app(state, config);
        // App created successfully
    }
}
