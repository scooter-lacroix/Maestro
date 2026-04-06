//! HTTP routes for the gateway
//!
//! Provides REST endpoints for health, pairing, webhooks, and API access.

use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::agent::{
    ApprovalDecisionRequest, McpAuthSubmitRequest, McpServerRegisterRequest,
    PairingInitiateRequest, PairingVerifyRequest,
};
use crate::agent_runtime::{self, McpConnectOutcome};
use crate::state::{scopes, GatewayState};

#[cfg(test)]
/// Generate a random number in the range [0, upper) using rejection sampling
/// to avoid modulo bias.
///
/// This ensures a uniform distribution even when the range doesn't evenly
/// divide the random number generator's output space.
fn random_uniform(upper: u32) -> u32 {
    assert!(upper > 0, "upper bound must be positive");
    assert!(
        upper <= u32::MAX / 2,
        "upper bound too large for rejection sampling"
    );

    // Calculate the threshold for rejection
    // We want to reject any value >= threshold to avoid bias
    let threshold = u32::MAX - (u32::MAX % upper);

    loop {
        let val = rand::random::<u32>();
        if val < threshold {
            return val % upper;
        }
        // Reject and retry to avoid bias
    }
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub uptime_secs: u64,
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

/// Dashboard overview response
#[derive(Debug, Serialize)]
pub struct DashboardOverview {
    pub uptime_secs: u64,
    pub connections: usize,
    pub cron_jobs: usize,
    pub mcp_servers: McpServersStatus,
    pub sandbox: SandboxStatus,
}

/// MCP servers status
#[derive(Debug, Serialize)]
pub struct McpServersStatus {
    pub registered: usize,
    pub connected: usize,
}

/// Sandbox status
#[derive(Debug, Serialize)]
pub struct SandboxStatus {
    pub autonomy_level: String,
    pub network_enabled: bool,
    pub runtimes: Vec<String>,
}

/// Create the HTTP routes router (stateless, caller must apply state)
pub fn create_routes() -> Router<Arc<GatewayState>> {
    Router::new()
        // Health and status
        .route("/health", get(handle_health))
        .route("/api/status", get(handle_api_status))
        // Dashboard
        .route("/api/dashboard", get(handle_dashboard))
        .route("/api/dashboard/jobs", get(handle_dashboard_jobs))
        .route("/api/dashboard/approvals", get(handle_dashboard_approvals))
        .route(
            "/api/dashboard/approvals/{id}",
            post(handle_dashboard_approval_decision),
        )
        // Pairing
        .route("/pair", post(handle_pair))
        .route("/pair/verify", post(handle_pair_verify))
        .route("/api/pairings", get(handle_pairings))
        .route("/api/tokens", get(handle_tokens))
        .route("/api/tokens/{id}", delete(handle_token_revoke))
        // Webhooks
        .route("/webhook", post(handle_webhook))
        // Agent Session API (new agent endpoints)
        .route("/api/agent/sessions", get(handle_agent_session_list))
        .route("/api/agent/sessions", post(handle_agent_session_create))
        .route("/api/agent/sessions/{id}", get(handle_agent_session_get))
        .route(
            "/api/agent/sessions/{id}",
            delete(handle_agent_session_delete),
        )
        .route("/api/agent/execute", post(handle_agent_execute))
        // Legacy Session API
        .route("/api/session", get(handle_session_list))
        .route("/api/session/{id}", get(handle_session_get))
        // MCP API
        .route("/api/mcp/servers", get(handle_mcp_servers))
        .route("/api/mcp/servers", post(handle_mcp_server_register))
        .route("/api/mcp/servers/{name}", delete(handle_mcp_server_remove))
        .route("/api/mcp/servers/{name}/connect", post(handle_mcp_connect))
        .route(
            "/api/mcp/servers/{name}/disconnect",
            post(handle_mcp_disconnect),
        )
        .route("/api/mcp/auth/{request_id}", post(handle_mcp_auth_submit))
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
pub async fn handle_api_status(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::SYSTEM))
    {
        return error.into_response();
    }

    let methods: Vec<String> = state
        .method_registry
        .list_methods()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let connections = state
        .connection_count
        .load(std::sync::atomic::Ordering::Relaxed);

    Json(ApiStatus {
        gateway: "maestro-gateway",
        connections,
        methods,
    })
    .into_response()
}

/// Pairing initiation
pub async fn handle_pair(
    State(state): State<Arc<GatewayState>>,
    Json(req): Json<PairingInitiateRequest>,
) -> impl IntoResponse {
    debug!("Pairing request from device: {:?}", req.device_name);
    Json(agent_runtime::initiate_pairing(&state, &req))
}

/// Pairing verification
pub async fn handle_pair_verify(
    State(state): State<Arc<GatewayState>>,
    Json(req): Json<PairingVerifyRequest>,
) -> impl IntoResponse {
    match agent_runtime::verify_pairing_code(&state, &req) {
        Ok(response) => Json(response).into_response(),
        Err(error) => error.into_response(),
    }
}

pub async fn handle_pairings(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::SYSTEM))
    {
        return error.into_response();
    }

    Json(agent_runtime::list_pairings(&state)).into_response()
}

pub async fn handle_tokens(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::SYSTEM))
    {
        return error.into_response();
    }

    Json(agent_runtime::list_tokens(&state)).into_response()
}

pub async fn handle_token_revoke(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::SYSTEM))
    {
        return error.into_response();
    }

    match agent_runtime::revoke_token(&state, &id) {
        Ok(response) => Json(response).into_response(),
        Err(error) => error.into_response(),
    }
}

/// Webhook receiver
pub async fn handle_webhook(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<WebhookPayload>,
) -> impl IntoResponse {
    debug!(
        "Webhook received: {} from {}",
        payload.event, payload.source
    );

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
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::SESSIONS))
    {
        return error.into_response();
    }

    Json(agent_runtime::list_sessions(&state)).into_response()
}

/// Get a specific session
pub async fn handle_session_get(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::SESSIONS))
    {
        return error.into_response();
    }

    match agent_runtime::get_session_info(&state, &id) {
        Some(info) => Json(info).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session not found"})),
        )
            .into_response(),
    }
}

/// List MCP servers
pub async fn handle_mcp_servers(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::TOOLS))
    {
        return error.into_response();
    }

    Json(agent_runtime::list_mcp_servers(&state).await).into_response()
}

pub async fn handle_mcp_server_register(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    Json(req): Json<McpServerRegisterRequest>,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::TOOLS))
    {
        return error.into_response();
    }

    match agent_runtime::register_or_update_mcp_server(&state, &req).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => error.into_response(),
    }
}

pub async fn handle_mcp_server_remove(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::TOOLS))
    {
        return error.into_response();
    }

    match agent_runtime::remove_mcp_server(&state, &name).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => error.into_response(),
    }
}

/// Connect to an MCP server
pub async fn handle_mcp_connect(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::TOOLS))
    {
        return error.into_response();
    }

    match agent_runtime::connect_mcp_server(state.clone(), &name).await {
        Ok(McpConnectOutcome::Connected) => {
            (StatusCode::OK, Json(serde_json::json!({"connected": true}))).into_response()
        }
        Ok(McpConnectOutcome::AuthRequired(auth)) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "connected": false,
                "auth_required": true,
                "auth": auth,
            })),
        )
            .into_response(),
        Err(error) => error.into_response(),
    }
}

/// Submit an MCP/tool auth token for a pending auth request.
pub async fn handle_mcp_auth_submit(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    axum::extract::Path(request_id): axum::extract::Path<String>,
    Json(req): Json<McpAuthSubmitRequest>,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::TOOLS))
    {
        return error.into_response();
    }

    match agent_runtime::submit_mcp_auth(
        &state,
        &request_id,
        agent_runtime::gateway_auth_token(req.token, req.token_type),
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => error.into_response(),
    }
}

/// Disconnect from an MCP server
pub async fn handle_mcp_disconnect(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::TOOLS))
    {
        return error.into_response();
    }

    match agent_runtime::disconnect_mcp_server(&state, &name).await {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"disconnected": true})),
        )
            .into_response(),
        Err(error) => error.into_response(),
    }
}

/// List cron jobs
pub async fn handle_cron_jobs(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::CRON))
    {
        return error.into_response();
    }

    Json(state.cron_jobs.clone()).into_response()
}

/// Create a cron job
pub async fn handle_cron_create(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    Json(_job): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::CRON))
    {
        return error.into_response();
    }

    let job: maestro_core::CronJob = match serde_json::from_value(_job) {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Invalid cron job definition: {}", e)})),
            )
            .into_response();
        }
    };

    // TODO: Route parsed job to a full async CronService writer once established in GatewayState
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "Job storage not yet fully integrated",
            "parsed_job_id": job.id
        })),
    )
        .into_response()
}

/// Dashboard overview
pub async fn handle_dashboard(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::SYSTEM))
    {
        return error.into_response();
    }

    let (registered, connected) = state.mcp_manager.try_get_status();
    let policy = state.sandbox_manager.default_policy();

    let autonomy_str = match policy.autonomy_level {
        maestro_core::AutonomyLevel::HumanApproval => "Human Approval",
        maestro_core::AutonomyLevel::Supervised => "Supervised",
        maestro_core::AutonomyLevel::Autonomous => "Autonomous",
    };

    Json(DashboardOverview {
        uptime_secs: state.start_time.elapsed().as_secs(),
        connections: state
            .connection_count
            .load(std::sync::atomic::Ordering::Relaxed),
        cron_jobs: state.cron_jobs.len(),
        mcp_servers: McpServersStatus {
            registered: registered.len(),
            connected: connected.len(),
        },
        sandbox: SandboxStatus {
            autonomy_level: autonomy_str.to_string(),
            network_enabled: policy.allow_network,
            runtimes: state
                .sandbox_manager
                .available_runtimes()
                .iter()
                .map(|s| s.to_string())
                .collect(),
        },
    })
    .into_response()
}

/// Dashboard job monitoring
pub async fn handle_dashboard_jobs(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::SYSTEM))
    {
        return error.into_response();
    }

    let now = chrono::Utc::now();

    // Return cron jobs with additional monitoring info
    let jobs: Vec<serde_json::Value> = state
        .cron_jobs
        .iter()
        .map(|job| {
            let schedule_str = match &job.schedule {
                maestro_core::Schedule::Cron { expr, .. } => expr.clone(),
                maestro_core::Schedule::At { at } => at.format("%Y-%m-%d %H:%M").to_string(),
                maestro_core::Schedule::Every { every_ms, .. } => format!("every {}ms", every_ms),
            };

            serde_json::json!({
                "id": job.id,
                "name": job.name,
                "schedule": schedule_str,
                "enabled": job.enabled,
                "job_type": match job.job_type {
                    maestro_core::JobType::Shell => "shell",
                    maestro_core::JobType::Agent => "agent",
                },
                "last_run": null,  // TODO: Map to actual runs from CronService when integrated
                "next_run": job.schedule.next_run(&now).map(|dt| dt.to_rfc3339()),
                "status": if job.enabled { "scheduled" } else { "paused" },
            })
        })
        .collect();

    Json(jobs).into_response()
}

/// Dashboard approval queue
pub async fn handle_dashboard_approvals(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::APPROVALS))
    {
        return error.into_response();
    }

    Json(agent_runtime::approval_queue(&state)).into_response()
}

pub async fn handle_dashboard_approval_decision(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<ApprovalDecisionRequest>,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::APPROVALS))
    {
        return error.into_response();
    }

    match agent_runtime::resolve_approval_request(&state, &id, &req) {
        Ok(response) => Json(response).into_response(),
        Err(error) => error.into_response(),
    }
}

// ============================================================================
// Agent API Handlers
// ============================================================================

/// List agent sessions (Rec-4)
pub async fn handle_agent_session_list(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::SESSIONS))
    {
        return error.into_response();
    }

    Json(agent_runtime::list_sessions(&state)).into_response()
}

/// Create a new agent session (Rec-4)
pub async fn handle_agent_session_create(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    Json(req): Json<crate::agent::SessionCreateRequest>,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::SESSIONS))
    {
        return error.into_response();
    }

    let created = agent_runtime::create_session(&state, &req);
    debug!("Created agent session: {}", created.session_id);
    Json(created).into_response()
}

/// Get a specific agent session (Rec-4)
pub async fn handle_agent_session_get(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::SESSIONS))
    {
        return error.into_response();
    }

    match agent_runtime::get_session_info(&state, &id) {
        Some(info) => Json(info).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session not found"})),
        )
            .into_response(),
    }
}

/// Delete an agent session (Rec-4)
pub async fn handle_agent_session_delete(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::SESSIONS))
    {
        return error.into_response();
    }

    let removed = agent_runtime::delete_session(&state, &id);

    if removed {
        debug!("Deleted agent session: {}", id);
        Json(crate::agent::SessionDeleteResponse { deleted: true }).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session not found"})),
        )
            .into_response()
    }
}

/// Execute an agent prompt (MED-7)
///
/// Flow:
/// 1. Authenticate (MED-8)
/// 2. Load existing session turns or start fresh (Rec-4)
/// 3. Build provider from config + request params
/// 4. Run `agent_loop` with a fresh `Thread` seeded with conversation history
/// 5. Persist updated turns back to the session store
/// 6. Return `AgentExecuteResponse`
pub async fn handle_agent_execute(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    Json(req): Json<crate::agent::AgentExecuteRequest>,
) -> impl IntoResponse {
    if let Err(error) =
        agent_runtime::verify_agent_auth_scoped(&state, &headers, None, Some(scopes::SESSIONS))
    {
        return error.into_response();
    }

    debug!("Agent execute: prompt_len={}", req.prompt.len());
    match agent_runtime::execute_agent_request(state.clone(), req).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => error.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_uniform_range() {
        // Test that generated values are within the expected range
        for _ in 0..1000 {
            let val = random_uniform(100);
            assert!(val < 100, "Value {} should be less than 100", val);
        }

        // Test with 1,000,000 (pairing code range)
        for _ in 0..100 {
            let val = random_uniform(1_000_000);
            assert!(
                val < 1_000_000,
                "Value {} should be less than 1,000,000",
                val
            );
        }
    }

    #[test]
    fn test_random_uniform_no_bias() {
        // Statistical test for uniform distribution
        // For a power-of-2 range, modulo bias doesn't occur
        // For 1,000,000, we use rejection sampling to avoid bias
        const SAMPLES: usize = 100_000;
        const BINS: usize = 10;
        const RANGE: u32 = 1_000_000;

        let mut bins = [0usize; BINS];
        let bin_size = (RANGE as usize) / BINS;

        for _ in 0..SAMPLES {
            let val = random_uniform(RANGE) as usize;
            let bin = val / bin_size;
            if bin < BINS {
                bins[bin] += 1;
            }
        }

        // Each bin should have approximately SAMPLES / BINS values
        // Allow for some variance (Chi-square test would be more rigorous)
        let expected = SAMPLES / BINS;
        let tolerance = (expected as f64) * 0.15; // 15% tolerance

        for (i, count) in bins.iter().enumerate() {
            let diff = (*count as f64) - (expected as f64);
            assert!(
                diff.abs() < tolerance,
                "Bin {} has {} samples, expected around {} (diff: {:.2})",
                i,
                count,
                expected,
                diff
            );
        }
    }

    #[test]
    #[should_panic(expected = "upper bound must be positive")]
    fn test_random_uniform_zero_panics() {
        random_uniform(0);
    }

    #[test]
    fn test_random_uniform_edge_cases() {
        // Test with upper = 1 (should always return 0)
        for _ in 0..10 {
            assert_eq!(random_uniform(1), 0);
        }

        // Test with upper = 2 (should return 0 or 1)
        for _ in 0..100 {
            let val = random_uniform(2);
            assert!(val < 2);
        }
    }
}
