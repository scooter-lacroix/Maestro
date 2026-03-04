//! HTTP routes for the gateway
//!
//! Provides REST endpoints for health, pairing, webhooks, and API access.

use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::state::GatewayState;

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
        // Pairing
        .route("/pair", post(handle_pair))
        .route("/pair/verify", post(handle_pair_verify))
        // Webhooks
        .route("/webhook", post(handle_webhook))
        // Agent Session API (new agent endpoints)
        .route("/api/agent/sessions", get(handle_agent_session_list))
        .route("/api/agent/sessions", post(handle_agent_session_create))
        .route("/api/agent/sessions/{id}", get(handle_agent_session_get))
        .route("/api/agent/sessions/{id}", delete(handle_agent_session_delete))
        .route("/api/agent/execute", post(handle_agent_execute))
        // Legacy Session API
        .route("/api/session", get(handle_session_list))
        .route("/api/session/{id}", get(handle_session_get))
        // MCP API
        .route("/api/mcp/servers", get(handle_mcp_servers))
        .route("/api/mcp/servers/{name}/connect", post(handle_mcp_connect))
        .route(
            "/api/mcp/servers/{name}/disconnect",
            post(handle_mcp_disconnect),
        )
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
}

/// Pairing initiation
pub async fn handle_pair(
    State(_state): State<Arc<GatewayState>>,
    Json(req): Json<PairRequest>,
) -> impl IntoResponse {
    debug!("Pairing request from device: {:?}", req.device_name);

    // Generate a pairing code if not provided
    let code = req.code.unwrap_or_else(|| {
        // Generate 6-digit code using rejection sampling for uniform distribution
        format!("{:06}", random_uniform(1_000_000))
    });

    // In a real implementation, this would:
    // 1. Store the pending pairing
    // 2. Require confirmation via another channel (e.g., TUI)
    // 3. Return a session token upon confirmation

    Json(PairResponse {
        paired: false,
        session_id: None,
        message: format!(
            "Pairing initiated. Enter code {} on the device to confirm.",
            code
        ),
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
pub async fn handle_session_list(State(_state): State<Arc<GatewayState>>) -> impl IntoResponse {
    // TODO: Implement session listing from persistence
    Json(Vec::<serde_json::Value>::new())
}

/// Get a specific session
pub async fn handle_session_get(
    State(_state): State<Arc<GatewayState>>,
    axum::extract::Path(_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    // TODO: Implement session retrieval from persistence
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "Not found"})),
    )
}

/// List MCP servers
pub async fn handle_mcp_servers(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
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
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"disconnected": true})),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

/// List cron jobs
pub async fn handle_cron_jobs(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
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

/// Dashboard overview
pub async fn handle_dashboard(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
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
}

/// Dashboard job monitoring
pub async fn handle_dashboard_jobs(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
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
                "last_run": null,  // TODO: Track actual runs
                "next_run": null,  // TODO: Calculate next run
                "status": if job.enabled { "scheduled" } else { "paused" },
            })
        })
        .collect();

    Json(jobs)
}

/// Dashboard approval queue
pub async fn handle_dashboard_approvals(
    State(_state): State<Arc<GatewayState>>,
) -> impl IntoResponse {
    // TODO: Wire up to approval manager
    // For now, return empty queue
    Json(serde_json::json!({
        "pending": [],
        "count": 0,
    }))
}

// ============================================================================
// Agent API — Authentication helper (MED-8)
// ============================================================================

/// Verify the Bearer token on agent API requests.
///
/// If `GatewayConfig::agent_api_key` is `None`, all requests are allowed
/// (useful for local development).  When a key is configured, the
/// `Authorization: Bearer <key>` header must match exactly.
fn verify_agent_auth(state: &GatewayState, headers: &HeaderMap) -> Result<(), Response> {
    let Some(ref required) = state.config.agent_api_key else {
        return Ok(()); // No key configured → open access
    };

    let bearer = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    match bearer {
        Some(token) if token == required.as_str() => Ok(()),
        Some(_) => Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid API key"})),
        )
            .into_response()),
        None => Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Authorization: Bearer <key> header required"})),
        )
            .into_response()),
    }
}

// ============================================================================
// Agent API — Provider factory (MED-7)
// ============================================================================

/// Build a `maestro_claw::agent::Provider` from gateway config + request params.
///
/// The provider name is taken from `req.provider`, falling back to
/// `config.default_llm_provider`.  The model is taken from `req.model`,
/// falling back to `config.default_model`, then to a provider-specific
/// built-in default.
fn build_agent_provider(
    config: &crate::state::GatewayConfig,
    req: &crate::agent::AgentExecuteRequest,
) -> Result<Arc<dyn maestro_claw::agent::Provider>, Response> {
    use maestro_claw::{
        AnthropicConfig, AnthropicProvider, OpenAIConfig, OpenAIProvider, ProviderAdapter,
    };

    let provider_name = req
        .provider
        .as_deref()
        .unwrap_or(config.default_llm_provider.as_str());

    let err = |msg: &str| -> Response {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": msg})),
        )
            .into_response()
    };

    match provider_name {
        "openai" => {
            let api_key = config
                .openai_api_key
                .as_deref()
                .ok_or_else(|| err("OpenAI API key not configured (set openai_api_key)"))?;
            let model = req
                .model
                .as_deref()
                .or(config.default_model.as_deref())
                .unwrap_or("gpt-4o");
            let inner = OpenAIProvider::new(OpenAIConfig::new(
                api_key.to_string(),
                model.to_string(),
            ))
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response()
            })?;
            Ok(Arc::new(ProviderAdapter::new(Arc::new(inner))))
        }
        "anthropic" => {
            let api_key = config
                .anthropic_api_key
                .as_deref()
                .ok_or_else(|| err("Anthropic API key not configured (set anthropic_api_key)"))?;
            let model = req
                .model
                .as_deref()
                .or(config.default_model.as_deref())
                .unwrap_or("claude-3-5-sonnet-20241022");
            let inner = AnthropicProvider::new(AnthropicConfig::new(
                api_key.to_string(),
                model.to_string(),
            ))
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response()
            })?;
            Ok(Arc::new(ProviderAdapter::new(Arc::new(inner))))
        }
        unknown => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Unknown provider '{}'. Supported: openai, anthropic", unknown)
            })),
        )
            .into_response()),
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
    if let Err(r) = verify_agent_auth(&state, &headers) {
        return r;
    }

    let sessions: Vec<crate::agent::SessionInfo> = state
        .session_store
        .iter()
        .map(|entry| entry.value().info.clone())
        .collect();

    Json(crate::agent::SessionListResponse::from_sessions(sessions)).into_response()
}

/// Create a new agent session (Rec-4)
pub async fn handle_agent_session_create(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    Json(req): Json<crate::agent::SessionCreateRequest>,
) -> impl IntoResponse {
    if let Err(r) = verify_agent_auth(&state, &headers) {
        return r;
    }

    let session_id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    let info = crate::agent::SessionInfo {
        id: session_id.clone(),
        thread_count: 0,
        turn_count: 0,
        created_at: created_at.clone(),
        status: "idle".to_string(),
    };

    state.session_store.insert(
        session_id.clone(),
        crate::state::StoredAgentSession {
            info: info.clone(),
            turns: Vec::new(),
            provider: req.provider.clone(),
            model: req.model.clone().unwrap_or_default(),
        },
    );

    debug!("Created agent session: {}", session_id);

    Json(crate::agent::SessionCreateResponse {
        session_id,
        created_at,
    })
    .into_response()
}

/// Get a specific agent session (Rec-4)
pub async fn handle_agent_session_get(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    if let Err(r) = verify_agent_auth(&state, &headers) {
        return r;
    }

    match state.session_store.get(&id) {
        Some(entry) => Json(entry.value().info.clone()).into_response(),
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
    if let Err(r) = verify_agent_auth(&state, &headers) {
        return r;
    }

    let removed = state.session_store.remove(&id).is_some();

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
    // MED-8: Authenticate
    if let Err(r) = verify_agent_auth(&state, &headers) {
        return r;
    }

    debug!("Agent execute: prompt_len={}", req.prompt.len());

    // ------------------------------------------------------------------
    // Build provider (MED-7)
    // ------------------------------------------------------------------
    let provider = match build_agent_provider(&state.config, &req) {
        Ok(p) => p,
        Err(r) => return r,
    };

    // EDGE-4: Garbage-collect stale/excess sessions before creating new ones
    state.gc_sessions();

    // ------------------------------------------------------------------
    // Resolve session and extract prior turns (Rec-4)
    //
    // We extract turns without holding any DashMap lock so the lock is
    // dropped before we enter the (potentially long) agent_loop await.
    // ------------------------------------------------------------------
    let (session_id, prior_turns) = if let Some(ref sid) = req.session_id {
        // Look up existing session
        let turns = match state.session_store.get(sid) {
            Some(entry) => entry.value().turns.clone(),
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "Session not found"})),
                )
                    .into_response();
            }
        };
        (sid.clone(), turns)
        // DashMap Ref dropped here before .await
    } else {
        // Auto-create a new session
        let new_id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();
        let info = crate::agent::SessionInfo {
            id: new_id.clone(),
            thread_count: 1,
            turn_count: 0,
            created_at,
            status: "active".to_string(),
        };
        state.session_store.insert(
            new_id.clone(),
            crate::state::StoredAgentSession {
                info,
                turns: Vec::new(),
                provider: req
                    .provider
                    .clone()
                    .unwrap_or_else(|| state.config.default_llm_provider.clone()),
                model: req.model.clone().unwrap_or_default(),
            },
        );
        (new_id, Vec::new())
    };

    // ------------------------------------------------------------------
    // Build a Thread hydrated with prior conversation history
    // ------------------------------------------------------------------
    let mut thread = maestro_claw::Thread::new(session_id.clone());
    for turn in prior_turns {
        thread.add_turn(turn);
    }
    thread.add_turn(maestro_claw::Turn::new(
        maestro_claw::TurnRole::User,
        req.prompt.clone(),
    ));

    let thread_id = thread.id().to_string();

    // ------------------------------------------------------------------
    // Broadcast "started" event
    // ------------------------------------------------------------------
    let prompt_preview = {
        const PREVIEW_CHARS: usize = 97;
        if req.prompt.chars().count() > PREVIEW_CHARS {
            let truncated: String = req.prompt.chars().take(PREVIEW_CHARS).collect();
            format!("{}...", truncated)
        } else {
            req.prompt.clone()
        }
    };
    let _ = state.event_bus.send(crate::protocol::EventFrame::new(
        "agent.execute.started",
        Some(serde_json::json!({
            "session_id": session_id,
            "thread_id": thread_id,
            "prompt_preview": prompt_preview,
        })),
        Some(state.next_seq()),
    ));

    // ------------------------------------------------------------------
    // Run the agent loop (may take many seconds for multi-turn sessions)
    // ------------------------------------------------------------------
    let max_turns = req.max_turns.unwrap_or(20);
    let config = maestro_claw::AgentConfig::default().with_max_turns(max_turns);
    let tools = Arc::new(maestro_claw::ToolRegistry::new());
    let hooks = Arc::new(maestro_claw::HookSystem::new());

    let agent_result = maestro_claw::agent_loop(&mut thread, provider, tools, hooks, config).await;

    // ------------------------------------------------------------------
    // Persist updated turns back to the session store (Rec-4)
    //
    // This is done without holding any lock during the await above; now
    // we briefly acquire the DashMap shard lock just to write.
    // ------------------------------------------------------------------
    {
        let updated_turns = thread.turns.clone();
        if let Some(mut entry) = state.session_store.get_mut(&session_id) {
            entry.value_mut().turns = updated_turns;
            entry.value_mut().info.turn_count = thread.turns.len();
            entry.value_mut().info.thread_count = 1;
            entry.value_mut().info.status = "idle".to_string();
        }
    }

    // ------------------------------------------------------------------
    // Build and return response
    // ------------------------------------------------------------------
    match agent_result {
        Ok(result) => {
            // Broadcast "completed" event
            let _ = state.event_bus.send(crate::protocol::EventFrame::new(
                "agent.execute.completed",
                Some(serde_json::json!({
                    "session_id": session_id,
                    "thread_id": thread_id,
                    "turns_used": result.total_turns,
                    "tool_calls": result.tool_calls_executed,
                    "completed_normally": result.completed_normally,
                })),
                Some(state.next_seq()),
            ));

            let response = crate::agent::AgentExecuteResponse {
                session_id,
                thread_id,
                content: result.content().to_string(),
                turns_used: result.total_turns,
                tool_calls: result.tool_calls_executed,
                completed_normally: result.completed_normally,
                termination_reason: if !result.completed_normally {
                    Some(result.termination_reason.clone())
                } else {
                    None
                },
            };
            Json(response).into_response()
        }
        Err(e) => {
            warn!("Agent loop error for session {}: {}", session_id, e);
            // Update session status to error
            if let Some(mut entry) = state.session_store.get_mut(&session_id) {
                entry.value_mut().info.status = "error".to_string();
            }
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
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
