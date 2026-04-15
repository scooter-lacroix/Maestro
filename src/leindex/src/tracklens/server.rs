// TrackLens Server - Axum-based HTTP server for review UI
//
// This module provides:
// - Axum server with approve/deny endpoints
// - HTML injection for browser-based review
// - WebSocket support for real-time updates
// - Integration with walkthrough generator
// - Token-based authentication for decision endpoint

use axum::{
    extract::{Json, State},
    http::{header, HeaderValue, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::time::{timeout, Duration};
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};

use super::types::{ReviewMode, TrackLensDecision, TrackLensPhase};

// ─── Server Configuration ─────────────────────────────────────────────────────

/// TrackLens server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Port to listen on
    pub port: u16,
    /// Host to bind to
    pub host: String,
    /// Enable browser opening
    pub open_browser: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 0, // Random port
            host: "127.0.0.1".to_string(),
            open_browser: true,
        }
    }
}

// ─── Server State ─────────────────────────────────────────────────────────────

/// Shared server state
#[derive(Debug)]
pub struct ServerState {
    /// Current review content
    pub content: Arc<std::sync::RwLock<Option<ReviewContent>>>,
    /// User decision transmitter
    pub decision_tx: watch::Sender<Option<TrackLensDecision>>,
    /// User decision receiver
    pub decision_rx: watch::Receiver<Option<TrackLensDecision>>,
    /// Client readiness transmitter
    pub client_ready_tx: watch::Sender<bool>,
    /// Client readiness receiver
    pub client_ready_rx: watch::Receiver<bool>,
    /// Timeout deadline (Unix timestamp in seconds)
    pub deadline_tx: watch::Sender<u64>,
    /// Timeout deadline receiver
    pub deadline_rx: watch::Receiver<u64>,
    /// Phase tracking transmitter
    pub phase_tx: watch::Sender<TrackLensPhase>,
    /// Phase tracking receiver
    pub phase_rx: watch::Receiver<TrackLensPhase>,
    /// Review iteration counter
    pub iteration: Arc<AtomicU32>,
    /// Shutdown signal transmitter
    pub shutdown_tx: watch::Sender<bool>,
    /// Shutdown signal receiver
    pub shutdown_rx: watch::Receiver<bool>,
}

/// Review content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewContent {
    /// Review mode
    pub mode: ReviewMode,
    /// Content to review
    pub content: String,
    /// Metadata (track ID, document type, etc.)
    pub metadata: ReviewMetadata,
}

/// Review metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewMetadata {
    /// Track ID
    pub track_id: Option<String>,
    /// Document type (spec, plan, walkthrough)
    pub document_type: String,
    /// Origin (claude-code, opencode, pi-mono)
    pub origin: String,
}

// ─── Server ───────────────────────────────────────────────────────────────────

/// TrackLens Axum server
#[derive(Debug, Clone)]
pub struct TrackLensServer {
    /// Server configuration
    pub config: ServerConfig,
    /// Server state
    pub state: Arc<ServerState>,
}

impl TrackLensServer {
    /// Create a new TrackLens server
    pub fn new(config: ServerConfig) -> Self {
        // Create watch channel for decision updates
        let (decision_tx, decision_rx) = watch::channel(None);
        let (client_ready_tx, client_ready_rx) = watch::channel(false);
        // Initial deadline: 20 seconds from now (default timeout)
        let initial_deadline = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + 20;
        let (deadline_tx, deadline_rx) = watch::channel(initial_deadline);
        let (phase_tx, phase_rx) = watch::channel(TrackLensPhase::Launching);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Self {
            config,
            state: Arc::new(ServerState {
                content: Arc::new(std::sync::RwLock::new(None)),
                decision_tx,
                decision_rx,
                client_ready_tx,
                client_ready_rx,
                deadline_tx,
                deadline_rx,
                phase_tx,
                phase_rx,
                iteration: Arc::new(AtomicU32::new(0)),
                shutdown_tx,
                shutdown_rx,
            }),
        }
    }

    /// Start the server and return the URL
    pub async fn start(&self) -> anyhow::Result<String> {
        // Try to bind to a port in order of preference:
        // 1. If config.port is specified (non-zero), use that
        // 2. Try preferred ports: 3847, 17579, 3000
        // 3. Fall back to OS-assigned port (port 0)
        let listener = if self.config.port != 0 {
            TcpListener::bind(format!("{}:{}", self.config.host, self.config.port)).await
        } else {
            // Try preferred ports first for predictability
            let preferred_ports = [3847u16, 17579u16, 3000u16];
            let mut result = None;

            for port in &preferred_ports {
                match TcpListener::bind(format!("{}:{}", self.config.host, port)).await {
                    Ok(l) => {
                        result = Some(l);
                        break;
                    }
                    Err(_) => continue, // Port in use, try next
                }
            }

            // If all preferred ports are taken, use OS-assigned port
            match result {
                Some(l) => Ok(l),
                None => TcpListener::bind(format!("{}:0", self.config.host)).await,
            }
        }?;

        // Get the actual port from the bound socket (eliminates race condition)
        let port = listener.local_addr()?.port();
        let url = format!("http://{}:{}", self.config.host, port);

        // Build CORS origins with the actual port
        let origin_localhost = format!("http://localhost:{}", port)
            .parse::<HeaderValue>()
            .unwrap();
        let origin_127 = format!("http://127.0.0.1:{}", port)
            .parse::<HeaderValue>()
            .unwrap();

        // Find bundle directory for static assets
        let bundle_dir =
            find_bundle_dir().ok_or_else(|| anyhow::anyhow!("TrackLens UI bundle not found"))?;

        // Build the router with restrictive CORS and compression
        // Now using the dynamically determined port
        let mut app = Router::new()
            .route("/", get(index))
            .route("/api/decision", post(submit_decision))
            .route("/api/client-ready", post(mark_client_ready))
            .route("/api/extend-timeout", post(extend_timeout))
            .route("/api/content", get(get_content))
            .route("/api/plan", get(get_plan))
            .route("/api/diff", get(get_diff))
            .route("/api/status", get(get_status))
            .route("/api/vaults", get(get_vaults))
            .route("/api/agents", get(get_agents))
            .route("/api/phase", get(get_phase))
            .route("/api/phase", post(set_phase))
            .route("/api/content", post(update_content))
            .route("/api/reset", post(reset_review))
            .route("/api/shutdown", post(shutdown_server));

        // Add static asset serving if bundle directory found
        if bundle_dir.join("assets").exists() {
            app = app.nest_service("/assets", ServeDir::new(bundle_dir.join("assets")));
        }
        if bundle_dir.join("favicon.svg").exists() {
            app = app.route_service(
                "/favicon.svg",
                ServeFile::new(bundle_dir.join("favicon.svg")),
            );
        }

        let app = app
            .layer(
                // Restrictive CORS: only allow local requests for security
                // Uses the actual port assigned to the server
                CorsLayer::new()
                    .allow_origin(origin_localhost)
                    .allow_origin(origin_127)
                    .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                    .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]),
            )
            .layer(CompressionLayer::new()) // Compress HTML responses
            .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // Limit request body to 10MB for large payloads
            .with_state(self.state.clone());

        // Open browser if configured (non-blocking)
        if self.config.open_browser {
            let url_clone = url.clone();
            tokio::spawn(async move {
                if let Err(e) = open::that(&url_clone) {
                    eprintln!("[TrackLens] Failed to open browser: {}", e);
                }
            });
        }

        // Spawn server in background with graceful shutdown
        let mut shutdown_rx = self.state.shutdown_rx.clone();
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    // Check initial value before waiting for changes
                    if *shutdown_rx.borrow() {
                        return;
                    }
                    // Wait until shutdown signal is sent
                    loop {
                        shutdown_rx
                            .changed()
                            .await
                            .unwrap_or(());
                        if *shutdown_rx.borrow() {
                            break;
                        }
                    }
                })
                .await
            {
                eprintln!("Server error: {}", e);
            }
        });

        Ok(url)
    }

    /// Wait until the client reports that the UI has loaded.
    pub async fn wait_for_client_ready(&self, timeout_duration: Duration) -> anyhow::Result<()> {
        let mut rx = self.state.client_ready_rx.clone();
        if *rx.borrow() {
            return Ok(());
        }

        timeout(timeout_duration, async move {
            loop {
                rx.changed()
                    .await
                    .map_err(|e| anyhow::anyhow!("Client readiness channel closed: {}", e))?;
                if *rx.borrow() {
                    return Ok(());
                }
            }
        })
        .await
        .map_err(|_| anyhow::anyhow!("Timed out waiting for TrackLens UI readiness"))?
    }

    /// Set the review content.
    /// If the content starts with `<!-- tracklens:editable -->`, automatically
    /// sets the phase to `Editing` so the UI opens in edit mode.
    pub fn set_content(&self, content: ReviewContent) -> anyhow::Result<()> {
        let is_editable = content.content.starts_with("<!-- tracklens:editable -->");

        let mut state = self
            .state
            .content
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire lock: {}", e))?;
        *state = Some(content);

        // Auto-set phase to Editing for seed content
        if is_editable {
            let _ = self.state.phase_tx.send(TrackLensPhase::Editing);
        }

        Ok(())
    }

    /// Wait for user decision (blocking)
    pub async fn wait_for_decision(&self) -> anyhow::Result<TrackLensDecision> {
        let mut rx = self.state.decision_rx.clone();

        loop {
            // Wait for the channel to be updated
            rx.changed()
                .await
                .map_err(|e| anyhow::anyhow!("Channel closed: {}", e))?;

            // Check if we have a decision
            if let Some(decision) = rx.borrow().as_ref() {
                return Ok(decision.clone());
            }
        }
    }

    /// Set the current phase
    pub fn set_phase(&self, phase: TrackLensPhase) -> anyhow::Result<()> {
        self.state
            .phase_tx
            .send(phase)
            .map_err(|e| anyhow::anyhow!("Failed to set phase: {}", e))
    }

    /// Get the current phase
    pub fn current_phase(&self) -> TrackLensPhase {
        *self.state.phase_rx.borrow()
    }

    /// Wait for phase to change from the current value
    pub async fn wait_for_phase_change(&self) -> anyhow::Result<TrackLensPhase> {
        let mut rx = self.state.phase_rx.clone();
        rx.changed()
            .await
            .map_err(|e| anyhow::anyhow!("Phase channel closed: {}", e))?;
        let phase = *rx.borrow();
        Ok(phase)
    }

    /// Get current review iteration
    pub fn iteration(&self) -> u32 {
        self.state.iteration.load(Ordering::SeqCst)
    }

    /// Reset for a new review round: clear decision, increment iteration, update content, reset phase
    pub fn reset_for_resubmit(&self, new_content: Option<ReviewContent>) -> anyhow::Result<()> {
        // Clear the decision
        self.state
            .decision_tx
            .send(None)
            .map_err(|e| anyhow::anyhow!("Failed to clear decision: {}", e))?;

        // Increment iteration
        self.state.iteration.fetch_add(1, Ordering::SeqCst);

        // Update content if provided
        if let Some(content) = new_content {
            let mut guard = self
                .state
                .content
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire content lock: {}", e))?;
            *guard = Some(content);
        }

        // Reset phase to Reviewing
        self.state
            .phase_tx
            .send(TrackLensPhase::Reviewing)
            .map_err(|e| anyhow::anyhow!("Failed to reset phase: {}", e))?;

        Ok(())
    }

    /// Signal the server to shut down gracefully
    pub fn shutdown(&self) -> anyhow::Result<()> {
        self.state
            .shutdown_tx
            .send(true)
            .map_err(|e| anyhow::anyhow!("Failed to send shutdown signal: {}", e))
    }
}

// ─── HTTP Handlers ─────────────────────────────────────────────────────────────

/// Find the directory containing the TrackLens UI bundle
fn find_bundle_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_default();

    let candidate_dirs = vec![
        // Installed location (primary)
        format!("{home}/.maestro/tracklens"),
        // Next to the binary
        format!("{}", exe_dir.display()),
        // Project source tree locations (development)
        "packages/tracklens-editor/dist".to_string(),
        "apps/tracklens-hook/dist".to_string(),
        "crates/cli/dist".to_string(),
    ];

    for dir in candidate_dirs {
        let path = std::path::PathBuf::from(&dir);
        if path.join("index.html").exists()
            || path.join("editor.html").exists()
            || path.join("tracklens-editor.html").exists()
            || path.join("review.html").exists()
        {
            return Some(path);
        }
    }
    None
}

async fn index(State(state): State<Arc<ServerState>>) -> Html<String> {
    if let Some(bundle_dir) = find_bundle_dir() {
        // Determine which HTML file to serve based on review mode
        let html_names = if let Ok(content) = state.content.read() {
            if let Some(ref c) = *content {
                match c.mode {
                    ReviewMode::CodeReview => vec![
                        "review.html",
                        "index.html",
                        "editor.html",
                        "tracklens-editor.html",
                    ],
                    ReviewMode::Annotate => {
                        vec!["index.html", "editor.html", "tracklens-editor.html"]
                    }
                    ReviewMode::Review => {
                        vec!["index.html", "editor.html", "tracklens-editor.html"]
                    }
                }
            } else {
                vec!["index.html", "editor.html", "tracklens-editor.html"]
            }
        } else {
            vec!["index.html", "editor.html", "tracklens-editor.html"]
        };

        for name in html_names {
            let path = bundle_dir.join(name);
            if let Ok(mut content) = tokio::fs::read_to_string(path).await {
                let ready_script = r#"<script>
(function() {
    function markClientReady() {
        fetch("/api/client-ready", {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            }
        }).then(response => {
            if (response.ok) {
                console.log("TrackLens: Client ready signal sent");
            } else {
                console.error("TrackLens: Failed to mark client ready, status:", response.status);
            }
        }).catch(error => {
            console.error("TrackLens: Failed to send client ready signal:", error);
        });
    }

    // Wait for page load event to ensure React app has initialized
    // Module scripts load asynchronously, so we need to wait for 'load' event
    if (document.readyState === 'complete') {
        // Page already loaded, wait a bit for React to initialize
        setTimeout(markClientReady, 100);
    } else {
        // Wait for load event, then add a small delay for React
        window.addEventListener('load', function() {
            setTimeout(markClientReady, 100);
        });
    }
})();
</script>"#;
                // Inject script at the end of body (more reliable than head)
                content = content.replace("</body>", &format!("{}{}", ready_script, "</body>"));
                return Html(content);
            }
        }
    }

    Html(
        "<html><body><h1>TrackLens bundle missing</h1><p>The server should have failed before rendering this page.</p></body></html>"
            .to_string(),
    )
}

async fn submit_decision(
    State(state): State<Arc<ServerState>>,
    Json(decision): Json<TrackLensDecision>,
) -> Result<impl IntoResponse, StatusCode> {
    // Send decision via watch channel
    state
        .decision_tx
        .send(Some(decision))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

async fn mark_client_ready(
    State(state): State<Arc<ServerState>>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .client_ready_tx
        .send(true)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

/// Request to extend the review timeout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendTimeoutRequest {
    /// Additional minutes to add to the timeout
    pub minutes: u64,
}

async fn extend_timeout(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<ExtendTimeoutRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // Calculate new deadline: extend from current deadline or now, whichever is later
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    // Read current deadline from the receiver to extend from it
    let current_deadline = *state.deadline_rx.borrow();
    let base = current_deadline.max(now);
    let new_deadline = base + (req.minutes * 60);

    state
        .deadline_tx
        .send(new_deadline)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

async fn get_status() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

async fn get_vaults() -> Json<serde_json::Value> {
    // Return empty vaults for now, as this is project-specific
    // The UI handles empty results gracefully
    Json(serde_json::json!({ "vaults": [] }))
}

async fn get_agents() -> Json<serde_json::Value> {
    // Return standard Maestro agents for the UI settings dropdown
    Json(serde_json::json!({
        "agents": [
            { "id": "build", "name": "Build Agent" },
            { "id": "implement", "name": "Implementation Agent" },
            { "id": "qwen-coder", "name": "Qwen Coder" },
            { "id": "amp-code", "name": "Amp Code" },
            { "id": "rovo-dev", "name": "Rovo Dev" },
            { "id": "codex-reviewer", "name": "Codex Reviewer" }
        ]
    }))
}

async fn get_content(
    State(state): State<Arc<ServerState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let content = state
        .content
        .read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(c) = content.as_ref() {
        Ok(Json(c.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Get diff content - returns { "rawPatch": string, ... } format for review editor compatibility
/// This endpoint is used by the React review editor (code review mode)
async fn get_diff(State(state): State<Arc<ServerState>>) -> Json<serde_json::Value> {
    let content = state
        .content
        .read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);

    match content {
        Ok(content) => {
            if let Some(c) = content.as_ref() {
                // Match the format expected by the TypeScript review server
                Json(serde_json::json!({
                    "rawPatch": c.content,
                    "gitRef": "HEAD",
                    "origin": c.metadata.origin,
                    "diffType": "uncommitted",
                    "repoInfo": { "display": "local" }
                }))
            } else {
                Json(serde_json::json!({ "rawPatch": "" }))
            }
        }
        Err(_) => Json(serde_json::json!({ "rawPatch": "" })),
    }
}

/// Get plan content - returns { "plan": string } format for JS editor compatibility
async fn get_plan(State(state): State<Arc<ServerState>>) -> Json<serde_json::Value> {
    let content = state
        .content
        .read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);

    match content {
        Ok(content) => {
            if let Some(c) = content.as_ref() {
                Json(serde_json::json!({ "plan": c.content }))
            } else {
                Json(serde_json::json!({ "plan": "" }))
            }
        }
        Err(_) => Json(serde_json::json!({ "plan": "" })),
    }
}

// ─── Phase & Content Update Endpoints ──────────────────────────────────────────

/// Request to change the current review phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPhaseRequest {
    /// Target phase
    pub phase: TrackLensPhase,
}

/// Get the current review phase
async fn get_phase(State(state): State<Arc<ServerState>>) -> Json<serde_json::Value> {
    let phase = *state.phase_rx.borrow();
    Json(serde_json::json!({ "phase": phase }))
}

/// Set the current review phase
async fn set_phase(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<SetPhaseRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .phase_tx
        .send(req.phase)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

/// Update the review content (for multi-round editing)
async fn update_content(
    State(state): State<Arc<ServerState>>,
    Json(content): Json<ReviewContent>,
) -> Result<impl IntoResponse, StatusCode> {
    let is_editable = content.content.starts_with("<!-- tracklens:editable -->");
    let mut guard = state
        .content
        .write()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    *guard = Some(content);
    drop(guard);
    
    // Auto-set phase to Editing for seed content (matches set_content behavior)
    if is_editable {
        state
            .phase_tx
            .send(TrackLensPhase::Editing)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(StatusCode::OK)
}

/// Request to reset the review for a new round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetReviewRequest {
    /// Optional new content for the review round
    pub content: Option<ReviewContent>,
}

/// Reset the review for a new round: clears decision, increments iteration, resets phase
async fn reset_review(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<ResetReviewRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // Clear the decision
    state
        .decision_tx
        .send(None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Increment iteration
    state.iteration.fetch_add(1, Ordering::SeqCst);

    // Update content if provided
    if let Some(content) = req.content {
        let mut guard = state
            .content
            .write()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        *guard = Some(content);
    }

    // Reset phase to Reviewing
    state
        .phase_tx
        .send(TrackLensPhase::Reviewing)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let iteration = state.iteration.load(Ordering::SeqCst);
    Ok(Json(serde_json::json!({ "status": "reset", "iteration": iteration })))
}

/// Shut down the server gracefully
async fn shutdown_server(
    State(state): State<Arc<ServerState>>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .shutdown_tx
        .send(true)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "status": "shutting_down" })))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracklens::types::{DecisionBehavior, TrackLensPhase};
    use reqwest::Client;

    #[tokio::test]
    async fn test_server_creation() {
        let config = ServerConfig {
            port: 0,
            open_browser: false,
            ..Default::default()
        };
        let server = TrackLensServer::new(config);
        assert!(server.start().await.is_ok());
        // Verify phase defaults to Launching
        assert_eq!(server.current_phase(), TrackLensPhase::Launching);
        // Verify iteration starts at 0
        assert_eq!(server.iteration(), 0);
    }

    #[tokio::test]
    async fn test_phase_tracking() {
        let config = ServerConfig {
            port: 0,
            open_browser: false,
            ..Default::default()
        };
        let server = TrackLensServer::new(config);
        assert_eq!(server.current_phase(), TrackLensPhase::Launching);

        // Transition to Loading
        server.set_phase(TrackLensPhase::Loading).unwrap();
        assert_eq!(server.current_phase(), TrackLensPhase::Loading);

        // Transition to Reviewing
        server.set_phase(TrackLensPhase::Reviewing).unwrap();
        assert_eq!(server.current_phase(), TrackLensPhase::Reviewing);

        // Transition to Editing
        server.set_phase(TrackLensPhase::Editing).unwrap();
        assert_eq!(server.current_phase(), TrackLensPhase::Editing);

        // Transition to Decided
        server.set_phase(TrackLensPhase::Decided).unwrap();
        assert_eq!(server.current_phase(), TrackLensPhase::Decided);
    }

    #[tokio::test]
    async fn test_wait_for_phase_change() {
        let config = ServerConfig {
            port: 0,
            open_browser: false,
            ..Default::default()
        };
        let server = TrackLensServer::new(config);
        assert_eq!(server.current_phase(), TrackLensPhase::Launching);

        // Spawn a task that changes phase after a delay
        let server_clone = server.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            server_clone.set_phase(TrackLensPhase::Reviewing).unwrap();
        });

        let new_phase = server.wait_for_phase_change().await.unwrap();
        assert_eq!(new_phase, TrackLensPhase::Reviewing);
    }

    /// Cross-boundary decision integration test
    /// Validates: HTML injection → JS bootstraps → decision POST reaches Rust state
    #[tokio::test]
    async fn test_decision_flow_integration() {
        let config = ServerConfig {
            port: 0,
            open_browser: false,
            ..Default::default()
        };
        let server = TrackLensServer::new(config);

        // Start the server
        let server_url = server.start().await.expect("Server should start");

        // Fetch HTML and verify the readiness bootstrap script is injected.
        let client = Client::new();
        let html_response = client.get(&server_url).send().await.unwrap();
        let html = html_response.text().await.unwrap();

        assert!(
            html.contains("markClientReady"),
            "TrackLens bootstrap script should be injected into the HTML"
        );

        // Submit a decision and verify it flows through to the watch channel state.
        let decision_payload = serde_json::json!({
            "behavior": "allow",
            "annotations": null,
            "autonomy_mode": null,
            "feedback": null
        });

        let response = client
            .post(format!("{}/api/decision", server_url))
            .json(&decision_payload)
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200, "Decision post should be accepted");

        let decision = server.wait_for_decision().await.unwrap();
        assert_eq!(decision.behavior, DecisionBehavior::Allow);
    }

    /// Test that server startup allocates distinct URLs across repeated launches.
    #[tokio::test]
    async fn test_server_urls_are_unique() {
        let config = ServerConfig {
            port: 0,
            open_browser: false,
            ..Default::default()
        };

        let mut urls = std::collections::HashSet::new();

        // Create multiple servers and verify each launch gets its own reachable URL.
        for _ in 0..5 {
            let server = TrackLensServer::new(config.clone());
            let url = server.start().await.expect("Server should start");
            urls.insert(url);
        }

        assert_eq!(urls.len(), 5, "All server URLs should be unique");
    }

    /// Test GET /api/phase returns the current phase
    #[tokio::test]
    async fn test_get_phase_endpoint() {
        let config = ServerConfig {
            port: 0,
            open_browser: false,
            ..Default::default()
        };
        let server = TrackLensServer::new(config);
        let url = server.start().await.expect("Server should start");
        let client = Client::new();

        // Default phase is Launching
        let resp = client.get(format!("{}/api/phase", url)).send().await.unwrap();
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["phase"], "launching");

        // Set phase server-side and verify endpoint reflects it
        server.set_phase(TrackLensPhase::Reviewing).unwrap();
        let resp = client.get(format!("{}/api/phase", url)).send().await.unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["phase"], "reviewing");
    }

    /// Test POST /api/phase changes the phase via HTTP
    #[tokio::test]
    async fn test_set_phase_endpoint() {
        let config = ServerConfig {
            port: 0,
            open_browser: false,
            ..Default::default()
        };
        let server = TrackLensServer::new(config);
        let url = server.start().await.expect("Server should start");
        let client = Client::new();

        // Set phase to Editing via HTTP
        let resp = client
            .post(format!("{}/api/phase", url))
            .json(&serde_json::json!({ "phase": "editing" }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        // Verify server-side state updated
        assert_eq!(server.current_phase(), TrackLensPhase::Editing);

        // Verify GET reflects the change
        let resp = client.get(format!("{}/api/phase", url)).send().await.unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["phase"], "editing");
    }

    /// Test POST /api/content replaces review content
    #[tokio::test]
    async fn test_update_content_endpoint() {
        let config = ServerConfig {
            port: 0,
            open_browser: false,
            ..Default::default()
        };
        let server = TrackLensServer::new(config);
        let url = server.start().await.expect("Server should start");
        let client = Client::new();

        // Initially no content
        let resp = client.get(format!("{}/api/content", url)).send().await.unwrap();
        assert_eq!(resp.status(), 404);

        // POST new content
        let new_content = serde_json::json!({
            "mode": "review",
            "content": "# Revised Plan\n\nUpdated via POST /api/content",
            "metadata": {
                "track_id": "test-123",
                "document_type": "plan.md",
                "origin": "test"
            }
        });
        let resp = client
            .post(format!("{}/api/content", url))
            .json(&new_content)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        // Verify GET returns updated content
        let resp = client.get(format!("{}/api/content", url)).send().await.unwrap();
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["content"], "# Revised Plan\n\nUpdated via POST /api/content");
        assert_eq!(body["metadata"]["track_id"], "test-123");
    }

    /// Test POST /api/reset clears decision, increments iteration, resets phase
    #[tokio::test]
    async fn test_reset_review_endpoint() {
        let config = ServerConfig {
            port: 0,
            open_browser: false,
            ..Default::default()
        };
        let server = TrackLensServer::new(config);
        let url = server.start().await.expect("Server should start");
        let client = Client::new();

        // Set up initial state: phase = Editing, iteration = 0
        server.set_phase(TrackLensPhase::Editing).unwrap();
        assert_eq!(server.iteration(), 0);

        // Submit a decision to populate the channel
        let decision = serde_json::json!({
            "behavior": "deny",
            "annotations": null,
            "feedback": "Needs work"
        });
        client
            .post(format!("{}/api/decision", url))
            .json(&decision)
            .send()
            .await
            .unwrap();

        // Reset with new content
        let reset_payload = serde_json::json!({
            "content": {
                "mode": "review",
                "content": "# Revised Plan v2",
                "metadata": {
                    "track_id": "test-456",
                    "document_type": "plan.md",
                    "origin": "test"
                }
            }
        });
        let resp = client
            .post(format!("{}/api/reset", url))
            .json(&reset_payload)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["status"], "reset");
        assert_eq!(body["iteration"], 1);

        // Verify server state
        assert_eq!(server.iteration(), 1);
        assert_eq!(server.current_phase(), TrackLensPhase::Reviewing);

        // Verify content was updated
        let resp = client.get(format!("{}/api/content", url)).send().await.unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["content"], "# Revised Plan v2");
    }

    /// Test reset_for_resubmit method (programmatic reset)
    #[tokio::test]
    async fn test_reset_for_resubmit_method() {
        let config = ServerConfig {
            port: 0,
            open_browser: false,
            ..Default::default()
        };
        let server = TrackLensServer::new(config);

        // Initial state
        assert_eq!(server.iteration(), 0);
        assert_eq!(server.current_phase(), TrackLensPhase::Launching);

        // Set up some state
        server.set_phase(TrackLensPhase::Decided).unwrap();
        assert_eq!(server.current_phase(), TrackLensPhase::Decided);

        // Reset with new content
        let new_content = ReviewContent {
            mode: ReviewMode::Review,
            content: "# Updated content".to_string(),
            metadata: ReviewMetadata {
                track_id: Some("test".to_string()),
                document_type: "plan.md".to_string(),
                origin: "test".to_string(),
            },
        };
        server.reset_for_resubmit(Some(new_content)).unwrap();

        assert_eq!(server.iteration(), 1);
        assert_eq!(server.current_phase(), TrackLensPhase::Reviewing);

        // Reset again without content
        server.reset_for_resubmit(None).unwrap();
        assert_eq!(server.iteration(), 2);
        assert_eq!(server.current_phase(), TrackLensPhase::Reviewing);
    }

    /// Test POST /api/shutdown triggers graceful shutdown
    #[tokio::test]
    async fn test_shutdown_endpoint() {
        let config = ServerConfig {
            port: 0,
            open_browser: false,
            ..Default::default()
        };
        let server = TrackLensServer::new(config);
        let url = server.start().await.expect("Server should start");
        let client = Client::new();

        // Verify server is up before shutdown
        let resp = client.get(format!("{}/api/status", url)).send().await.unwrap();
        assert_eq!(resp.status(), 200);

        // Request shutdown
        let resp = client
            .post(format!("{}/api/shutdown", url))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["status"], "shutting_down");

        // Poll until server is shut down (with timeout)
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(2);
        loop {
            let result = client.get(format!("{}/api/status", url)).send().await;
            if result.is_err() {
                break; // Server is shut down
            }
            if start.elapsed() > timeout {
                panic!("Server did not shut down within {:?}", timeout);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Test shutdown() method (programmatic shutdown)
    #[tokio::test]
    async fn test_shutdown_method() {
        let config = ServerConfig {
            port: 0,
            open_browser: false,
            ..Default::default()
        };
        let server = TrackLensServer::new(config);
        let url = server.start().await.expect("Server should start");

        // Verify server is up
        let client = Client::new();
        let resp = client.get(format!("{}/api/status", url)).send().await.unwrap();
        assert_eq!(resp.status(), 200);

        // Trigger shutdown programmatically
        server.shutdown().unwrap();

        // Poll until server is shut down (with timeout)
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(2);
        loop {
            let result = client.get(format!("{}/api/status", url)).send().await;
            if result.is_err() {
                break; // Server is shut down
            }
            if start.elapsed() > timeout {
                panic!("Server did not shut down within {:?}", timeout);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Test set_content with editable marker auto-sets phase to Editing
    #[tokio::test]
    async fn test_set_content_editable_marker() {
        let config = ServerConfig {
            port: 0,
            open_browser: false,
            ..Default::default()
        };
        let server = TrackLensServer::new(config);

        // Normal content should not change phase
        let normal_content = ReviewContent {
            mode: ReviewMode::Review,
            content: "# Normal Plan\n\nNo marker here.".to_string(),
            metadata: ReviewMetadata {
                track_id: Some("test".to_string()),
                document_type: "plan.md".to_string(),
                origin: "test".to_string(),
            },
        };
        server.set_content(normal_content).unwrap();
        assert_eq!(
            server.current_phase(),
            TrackLensPhase::Launching,
            "Normal content should not change phase from Launching"
        );

        // Editable content should auto-set phase to Editing
        let editable_content = ReviewContent {
            mode: ReviewMode::Review,
            content: "<!-- tracklens:editable -->\n# Seed Plan\n\nEdit this.".to_string(),
            metadata: ReviewMetadata {
                track_id: Some("test".to_string()),
                document_type: "plan.md".to_string(),
                origin: "test".to_string(),
            },
        };
        server.set_content(editable_content).unwrap();
        assert_eq!(
            server.current_phase(),
            TrackLensPhase::Editing,
            "Editable marker should auto-set phase to Editing"
        );
    }
}
