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
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};

use super::types::{ReviewMode, TrackLensDecision};

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
#[derive(Debug, Clone)]
pub struct ServerState {
    /// Current review content
    pub content: Arc<std::sync::RwLock<Option<ReviewContent>>>,
    /// User decision transmitter
    pub decision_tx: watch::Sender<Option<TrackLensDecision>>,
    /// User decision receiver
    pub decision_rx: watch::Receiver<Option<TrackLensDecision>>,
    /// Authentication token for decision endpoint
    pub auth_token: String,
    /// Client readiness signal
    pub client_ready_tx: watch::Sender<bool>,
    pub client_ready_rx: watch::Receiver<bool>,
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
        // Generate a secure random token for decision authentication
        let auth_token = Self::generate_auth_token();

        // Create watch channel for decision updates
        let (decision_tx, decision_rx) = watch::channel(None);
        let (client_ready_tx, client_ready_rx) = watch::channel(false);

        Self {
            config,
            state: Arc::new(ServerState {
                content: Arc::new(std::sync::RwLock::new(None)),
                decision_tx,
                decision_rx,
                auth_token,
                client_ready_tx,
                client_ready_rx,
            }),
        }
    }

    /// Generate a cryptographically secure authentication token
    fn generate_auth_token() -> String {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let bytes: [u8; 32] = rng.gen();
        // Encode as hex using format loop (hex crate not in dependencies)
        bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }

    /// Get the current auth token (for testing/debugging)
    pub fn auth_token(&self) -> String {
        self.state.auth_token.clone()
    }

    /// Start the server and return the URL
    pub async fn start(&self) -> anyhow::Result<String> {
        // Bind to port first to determine the actual port
        let port = if self.config.port == 0 {
            // Find available port
            portpicker::pick_unused_port().unwrap_or(3000)
        } else {
            self.config.port
        };

        let addr = format!("{}:{}", self.config.host, port);
        let listener = TcpListener::bind(&addr).await?;

        let url = format!("http://{}", addr);

        // Build CORS origins with the actual port
        let origin_localhost = format!("http://localhost:{}", port)
            .parse::<HeaderValue>()
            .unwrap();
        let origin_127 = format!("http://127.0.0.1:{}", port)
            .parse::<HeaderValue>()
            .unwrap();

        // Find bundle directory for static assets
        let bundle_dir = find_bundle_dir();

        // Build the router with restrictive CORS and compression
        // Now using the dynamically determined port
        let mut app = Router::new()
            .route("/", get(index))
            .route("/api/decision", post(submit_decision))
            .route("/api/content", get(get_content))
            .route("/api/plan", get(get_plan))
            .route("/api/status", get(get_status))
            .route("/api/vaults", get(get_vaults))
            .route("/api/agents", get(get_agents))
            .route("/api/client-ready", post(mark_client_ready));

        // Add static asset serving if bundle directory found
        if let Some(ref dir) = bundle_dir {
            if dir.join("assets").exists() {
                app = app.nest_service("/assets", ServeDir::new(dir.join("assets")));
            }
            if dir.join("favicon.svg").exists() {
                app = app.route_service("/favicon.svg", ServeFile::new(dir.join("favicon.svg")));
            }
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
            .layer(RequestBodyLimitLayer::new(1024 * 100)) // Limit request body to 100KB
            .with_state(self.state.clone());

        // Open browser if configured
        if self.config.open_browser {
            if let Err(e) = open::that(&url) {
                eprintln!("Failed to open browser: {}", e);
            }
        }

        // Spawn server in background
        let _state = self.state.clone();
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                eprintln!("Server error: {}", e);
            }
        });

        Ok(url)
    }

    /// Set the review content
    pub fn set_content(&self, content: ReviewContent) -> anyhow::Result<()> {
        let mut state = self
            .state
            .content
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire lock: {}", e))?;
        *state = Some(content);
        Ok(())
    }

    /// Wait for the client UI to report readiness
    pub async fn wait_for_client_ready(&self, timeout: std::time::Duration) -> bool {
        let mut rx = self.state.client_ready_rx.clone();
        tokio::select! {
            result = async {
                while !*rx.borrow_and_update() {
                    if rx.changed().await.is_err() {
                        return false;
                    }
                }
                true
            } => result,
            _ = tokio::time::sleep(timeout) => false,
        }
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
        {
            return Some(path);
        }
    }
    None
}

/// Generate the bootstrap script that injects auth token and client-ready signal
fn generate_bootstrap_script(auth_token: &str) -> String {
    format!(
        r#"<script>
window.TRACKLENS_AUTH_TOKEN = "{}";
document.addEventListener('DOMContentLoaded', function() {{
    fetch('/api/client-ready', {{ method: 'POST' }}).catch(function() {{}});
}});
</script>"#,
        auth_token
    )
}

async fn index(State(state): State<Arc<ServerState>>) -> Html<String> {
    if let Some(bundle_dir) = find_bundle_dir() {
        let html_names = ["index.html", "editor.html", "tracklens-editor.html"];
        for name in html_names {
            let path = bundle_dir.join(name);
            if let Ok(mut content) = tokio::fs::read_to_string(path).await {
                // Inject auth token and client-ready signal into HTML
                let bootstrap_script = generate_bootstrap_script(&state.auth_token);
                // Inject after <head> tag
                content = content.replace("<head>", &format!("<head>{}", bootstrap_script));
                return Html(content);
            }
        }
    }

    // Fallback placeholder if no HTML bundle found
    let bootstrap_script = generate_bootstrap_script(&state.auth_token);
    Html(format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>TrackLens Review</title>
    <style>
        body {{ font-family: system-ui; max-width: 800px; margin: 50px auto; padding: 20px; }}
        .warning {{ background: #fff3cd; border: 1px solid #ffc107; padding: 20px; border-radius: 4px; }}
        h1 {{ color: #333; }}
    </style>
    {}
</head>
<body>
    <h1>TrackLens Review</h1>
    <div class="warning">
        <p><strong>Review UI bundle not found.</strong></p>
        <p>Build the TrackLens editor bundle:</p>
        <pre>cargo build --package maestro-cli --bins</pre>
        <p>Or run: <code>bun run build</code> in packages/tracklens-editor</p>
    </div>
    <p>Server is running. API endpoints available:</p>
    <ul>
        <li><code>GET /api/content</code> - Get review content</li>
        <li><code>POST /api/decision</code> - Submit decision (requires auth token)</li>
    </ul>
</body>
</html>"#,
        bootstrap_script
    ))
}

async fn submit_decision(
    State(state): State<Arc<ServerState>>,
    headers: HeaderMap,
    Json(decision): Json<TrackLensDecision>,
) -> Result<impl IntoResponse, StatusCode> {
    // Check Authorization header
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let provided_token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        Some(h) => h,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    // Validate token
    if provided_token != state.auth_token {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Send decision via watch channel
    state
        .decision_tx
        .send(Some(decision))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

async fn mark_client_ready(State(state): State<Arc<ServerState>>) -> StatusCode {
    let _ = state.client_ready_tx.send(true);
    StatusCode::OK
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
    let agents = match discover_agents().await {
        Some(agents) if !agents.is_empty() => agents,
        _ => vec![
            serde_json::json!({ "id": "build", "name": "Build Agent" }),
            serde_json::json!({ "id": "implement", "name": "Implementation Agent" }),
            serde_json::json!({ "id": "qwen-coder", "name": "Qwen Coder" }),
            serde_json::json!({ "id": "amp-code", "name": "Amp Code" }),
            serde_json::json!({ "id": "rovo-dev", "name": "Rovo Dev" }),
            serde_json::json!({ "id": "codex-reviewer", "name": "Codex Reviewer" }),
        ],
    };
    Json(serde_json::json!({ "agents": agents }))
}

async fn discover_agents() -> Option<Vec<serde_json::Value>> {
    let home = std::env::var("HOME").ok()?;
    let agents_dir = std::path::PathBuf::from(home).join(".maestro/agents");
    let mut entries = tokio::fs::read_dir(&agents_dir).await.ok()?;
    let mut agents: Vec<serde_json::Value> = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Ok(file_type) = entry.file_type().await {
            if file_type.is_dir() {
                let id = entry.file_name().to_string_lossy().to_string();
                let name = id
                    .split('-')
                    .map(|w| {
                        let mut chars = w.chars();
                        match chars.next() {
                            Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                            None => String::new(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                agents.push(serde_json::json!({ "id": id, "name": name }));
            }
        }
    }

    agents.sort_by(|a, b| {
        a["id"].as_str().unwrap_or("").cmp(b["id"].as_str().unwrap_or(""))
    });
    Some(agents)
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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
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
    }

    /// Cross-boundary auth integration test
    /// Validates: Token injection → JS reads → JS sends Bearer → Rust validates
    #[tokio::test]
    async fn test_auth_flow_integration() {
        let config = ServerConfig {
            port: 0,
            open_browser: false,
            ..Default::default()
        };
        let server = TrackLensServer::new(config);

        // Start the server
        let server_url = server.start().await.expect("Server should start");

        // Extract auth token from HTML (simulating JS injection)
        let client = Client::new();
        let html_response = client.get(&server_url).send().await.unwrap();
        let html = html_response.text().await.unwrap();

        // Extract token from the injected script tag
        let token = html
            .find("window.TRACKLENS_AUTH_TOKEN = \"")
            .and_then(|pos| {
                let start = pos + "window.TRACKLENS_AUTH_TOKEN = \"".len();
                html[start..].find('"').map(|end| &html[start..start + end])
            });

        assert!(token.is_some(), "Auth token should be injected in HTML");

        let token = token.unwrap();

        // Verify token is not a short timestamp-based value
        assert!(
            token.len() >= 32,
            "Token should be at least 32 chars (cryptographically secure)"
        );

        // Test 1: Submit decision with valid token (simulating JS editor behavior)
        let decision_payload = serde_json::json!({
            "behavior": "allow",
            "annotations": null,
            "autonomy_mode": null
        });

        let response = client
            .post(format!("{}/api/decision", server_url))
            .header("Authorization", format!("Bearer {}", token))
            .json(&decision_payload)
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200, "Valid token should be accepted");

        // Test 2: Submit with wrong token should fail
        let response = client
            .post(format!("{}/api/decision", server_url))
            .header("Authorization", "Bearer wrong-token-12345")
            .json(&decision_payload)
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 401, "Wrong token should be rejected");

        // Test 3: Submit without token should fail
        let response = client
            .post(format!("{}/api/decision", server_url))
            .json(&decision_payload)
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 401, "Missing token should be rejected");
    }

    /// Test that tokens are unique across server restarts
    #[tokio::test]
    async fn test_token_uniqueness() {
        let config = ServerConfig {
            port: 0,
            open_browser: false,
            ..Default::default()
        };

        let mut tokens = std::collections::HashSet::new();

        // Create multiple servers and verify all tokens are unique
        for _ in 0..5 {
            let server = TrackLensServer::new(config.clone());
            let token = server.auth_token();
            tokens.insert(token);
        }

        assert_eq!(tokens.len(), 5, "All tokens should be unique");
    }
}
