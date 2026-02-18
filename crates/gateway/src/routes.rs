//! HTTP routes for the gateway
//!
//! Provides REST endpoints for health, pairing, webhooks, and API access.

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::state::GatewayState;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub uptime_secs: u64,
}

/// Pairing request
#[derive(Debug, Deserialize)]
pub struct PairRequest {
    pub device_name: Option<String>,
    pub code: Option<String>,
}

/// Pairing response
#[derive(Debug, Serialize)]
pub struct PairResponse {
    pub paired: bool,
    pub session_id: Option<String>,
    pub message: String,
}

/// Webhook payload
#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub source: String,
    pub event: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

/// API status response
#[derive(Debug, Serialize)]
pub struct ApiStatus {
    pub gateway: &'static str,
    pub connections: usize,
    pub methods: Vec<String>,
}

/// Create the HTTP routes router (stateless, caller must apply state)
pub fn create_routes() -> Router<Arc<GatewayState>> {
    Router::new()
        // Health and status
        .route("/health", get(handle_health))
        .route("/api/status", get(handle_api_status))

        // Pairing
        .route("/pair", post(handle_pair))
        .route("/pair/verify", post(handle_pair_verify))

        // Webhooks
        .route("/webhook", post(handle_webhook))

        // Session API
        .route("/api/session", get(handle_session_list))
        .route("/api/session/{id}", get(handle_session_get))

        // MCP API
        .route("/api/mcp/servers", get(handle_mcp_servers))
        .route("/api/mcp/servers/{name}/connect", post(handle_mcp_connect))
        .route("/api/mcp/servers/{name}/disconnect", post(handle_mcp_disconnect))

        // Cron API
        .route("/api/cron/jobs", get(handle_cron_jobs))
        .route("/api/cron/jobs", post(handle_cron_create))
}

/// Health check endpoint
pub async fn handle_health(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();

    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_secs: uptime,
    })
}

/// API status endpoint
pub async fn handle_api_status(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    let methods: Vec<String> = state.method_registry.list_methods().iter().map(|s| s.to_string()).collect();
    let connections = state.connection_count.load(std::sync::atomic::Ordering::Relaxed);

    Json(ApiStatus {
        gateway: "maestro-gateway",
        connections,
        methods,
    })
}

/// Pairing initiation
pub async fn handle_pair(
    State(_state): State<Arc<GatewayState>>,
    Json(req): Json<PairRequest>,
) -> impl IntoResponse {
    debug!("Pairing request from device: {:?}", req.device_name);

    // Generate a pairing code if not provided
    let code = req.code.unwrap_or_else(|| {
        // Generate 6-digit code
        format!("{:06}", rand::random::<u32>() % 1_000_000)
    });

    // In a real implementation, this would:
    // 1. Store the pending pairing
    // 2. Require confirmation via another channel (e.g., TUI)
    // 3. Return a session token upon confirmation

    Json(PairResponse {
        paired: false,
        session_id: None,
        message: format!("Pairing initiated. Enter code {} on the device to confirm.", code),
    })
}

/// Pairing verification
pub async fn handle_pair_verify(
    State(_state): State<Arc<GatewayState>>,
    Json(_req): Json<PairRequest>,
) -> impl IntoResponse {
    // In a real implementation, verify the pairing code and issue a session token
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(PairResponse {
            paired: false,
            session_id: None,
            message: "Pairing verification not yet implemented".to_string(),
        }),
    )
}

/// Webhook receiver
pub async fn handle_webhook(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<WebhookPayload>,
) -> impl IntoResponse {
    debug!("Webhook received: {} from {}", payload.event, payload.source);

    // Broadcast the webhook as an event
    let event = crate::protocol::EventFrame::new(
        format!("webhook.{}.{}", payload.source, payload.event),
        Some(payload.data),
        Some(state.next_seq()),
    );

    let _ = state.event_bus.send(event);

    (StatusCode::OK, Json(serde_json::json!({"received": true})))
}

/// List sessions
pub async fn handle_session_list(
    State(_state): State<Arc<GatewayState>>,
) -> impl IntoResponse {
    // TODO: Implement session listing from persistence
    Json(Vec::<serde_json::Value>::new())
}

/// Get a specific session
pub async fn handle_session_get(
    State(_state): State<Arc<GatewayState>>,
    axum::extract::Path(_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    // TODO: Implement session retrieval from persistence
    (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Not found"})))
}

/// List MCP servers
pub async fn handle_mcp_servers(
    State(state): State<Arc<GatewayState>>,
) -> impl IntoResponse {
    let (registered, connected) = state.mcp_manager.try_get_status();

    Json(serde_json::json!({
        "registered": registered,
        "connected": connected,
    }))
}

/// Connect to an MCP server
pub async fn handle_mcp_connect(
    State(state): State<Arc<GatewayState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    match state.mcp_manager.connect(&name).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"connected": true}))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

/// Disconnect from an MCP server
pub async fn handle_mcp_disconnect(
    State(state): State<Arc<GatewayState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    match state.mcp_manager.disconnect(&name).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"disconnected": true}))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

/// List cron jobs
pub async fn handle_cron_jobs(
    State(state): State<Arc<GatewayState>>,
) -> impl IntoResponse {
    Json(state.cron_jobs.clone())
}

/// Create a cron job
pub async fn handle_cron_create(
    State(_state): State<Arc<GatewayState>>,
    Json(_job): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Implement cron job creation
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"error": "Not yet implemented"})),
    )
}
