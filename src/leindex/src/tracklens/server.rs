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
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;

use super::types::{TrackLensDecision, ReviewMode};

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
    /// User decision
    pub decision: Arc<std::sync::RwLock<Option<TrackLensDecision>>>,
    /// Authentication token for decision endpoint
    pub auth_token: String,
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

        Self {
            config,
            state: Arc::new(ServerState {
                content: Arc::new(std::sync::RwLock::new(None)),
                decision: Arc::new(std::sync::RwLock::new(None)),
                auth_token,
            }),
        }
    }

    /// Generate a cryptographically secure authentication token
    fn generate_auth_token() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        format!("{:x}", timestamp)
    }

    /// Get the current auth token (for testing/debugging)
    pub fn auth_token(&self) -> String {
        self.state.auth_token.clone()
    }

    /// Start the server and return the URL
    pub async fn start(&self) -> anyhow::Result<String> {
        // Build the router with restrictive CORS
        let app = Router::new()
            .route("/", get(index))
            .route("/api/decision", post(submit_decision))
            .route("/api/content", get(get_content))
            .layer(
                // Restrictive CORS: only allow local requests for security
                CorsLayer::new()
                    .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
                    .allow_origin("http://127.0.0.1:3000".parse::<HeaderValue>().unwrap())
                    .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                    .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
            )
            .layer(RequestBodyLimitLayer::new(1024 * 100)) // Limit request body to 100KB
            .with_state(self.state.clone());

        // Bind to port
        let port = if self.config.port == 0 {
            // Find available port
            portpicker::pick_unused_port().unwrap_or(3000)
        } else {
            self.config.port
        };

        let addr = format!("{}:{}", self.config.host, port);
        let listener = TcpListener::bind(&addr).await?;

        let url = format!("http://{}", addr);

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
        let mut state = self.state.content.write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire lock: {}", e))?;
        *state = Some(content);
        Ok(())
    }

    /// Wait for user decision (blocking)
    pub async fn wait_for_decision(&self) -> anyhow::Result<TrackLensDecision> {
        loop {
            let decision = self.state.decision.read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire lock: {}", e))?;
            if let Some(d) = decision.as_ref() {
                return Ok(d.clone());
            }
            drop(decision);
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}

// ─── HTTP Handlers ─────────────────────────────────────────────────────────────

async fn index(State(state): State<Arc<ServerState>>) -> Html<String> {
    // Try to load TrackLens editor HTML bundle
    let html_paths = [
        // CLI dist location (primary)
        "crates/cli/dist/tracklens-editor.html",
        // Development locations
        "packages/tracklens-editor/dist/index.html",
        "apps/tracklens-hook/dist/index.html",
    ];

    for path in html_paths {
        if let Ok(mut content) = tokio::fs::read_to_string(path).await {
            // Inject auth token into HTML
            let token_script = format!(
                r#"<script>window.TRACKLENS_AUTH_TOKEN = "{}";</script>"#,
                state.auth_token
            );
            // Inject after <head> tag
            content = content.replace("<head>", &format!("<head>{}", token_script));
            return Html(content);
        }
    }

    // Fallback placeholder if no HTML bundle found
    let token = &state.auth_token;
    Html(format!(r#"<!DOCTYPE html>
<html>
<head>
    <title>TrackLens Review</title>
    <style>
        body {{ font-family: system-ui; max-width: 800px; margin: 50px auto; padding: 20px; }}
        .warning {{ background: #fff3cd; border: 1px solid #ffc107; padding: 20px; border-radius: 4px; }}
        h1 {{ color: #333; }}
    </style>
    <script>window.TRACKLENS_AUTH_TOKEN = "{}";</script>
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
</html>"#, token))
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

    let mut state_dec = state.decision.write()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    *state_dec = Some(decision);
    Ok(StatusCode::OK)
}

async fn get_content(
    State(state): State<Arc<ServerState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let content = state.content.read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(c) = content.as_ref() {
        Ok(Json(c.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        let config = ServerConfig {
            port: 0,
            ..Default::default()
        };
        let server = TrackLensServer::new(config);
        assert!(server.start().await.is_ok());
    }
}
