//! Maestro Integration Layer for tab-rs
//!
//! This crate provides a high-level API for Maestro to interact with
//! the forked tab-rs terminal multiplexer. It handles:
//! - Daemon lifecycle management
//! - WebSocket connection management
//! - Session creation and management
//! - Transparency support via OSC 111
//!
//! ## Architecture
//!
//! ```text
//! MaestroTabMultiplexer
//!         │
//!         ▼
//! maestro-integration
//!         │
//!         ├── tab_api (types, config, launch)
//!         └── tab_websocket (WebSocket protocol)
//!                 │
//!                 ▼
//!           tab-daemon (via WebSocket)
//! ```

pub mod pty;
pub mod session;
pub mod transparency;
pub mod websocket_bridge;

pub use pty::PtyBridge;
pub use session::{MaestroSession, SessionManager};
pub use transparency::{apply_transparency, TransparencyConfig};
pub use websocket_bridge::WebSocketPtyBridge;

use anyhow::Result;
use tab_api::client::Request;
use tab_api::config::DaemonConfig;
use tab_api::launch::launch_daemon;
use tokio::sync::mpsc;

/// The main integration client for Maestro-tab communication
pub struct MaestroTabClient {
    /// The daemon configuration (port, auth token, etc.)
    daemon_config: DaemonConfig,
    /// Channel for sending requests to the daemon
    request_tx: mpsc::Sender<Request>,
    /// Channel for receiving responses from the daemon
    response_rx: mpsc::Receiver<Request>,
}

impl MaestroTabClient {
    /// Create a new client, launching the daemon if necessary
    pub async fn new() -> Result<Self> {
        let daemon_config = launch_daemon().await?;
        tracing::info!("Connected to tab-daemon on port {}", daemon_config.port);

        // TODO: Establish WebSocket connection to /cli endpoint
        let (request_tx, response_rx) = mpsc::channel(100);

        Ok(Self {
            daemon_config,
            request_tx,
            response_rx,
        })
    }

    /// Get the daemon configuration
    pub fn daemon_config(&self) -> &DaemonConfig {
        &self.daemon_config
    }

    /// Check if the daemon is running
    pub fn is_daemon_running(&self) -> bool {
        self.daemon_config.pid > 0
    }

    /// Send a request to the daemon
    pub async fn send_request(&self, request: Request) -> Result<()> {
        self.request_tx.send(request).await?;
        Ok(())
    }
}

/// Errors specific to the Maestro-tab integration
#[derive(Debug, thiserror::Error)]
pub enum MaestroTabError {
    #[error("Daemon not running")]
    DaemonNotRunning,

    #[error("WebSocket connection failed: {0}")]
    WebSocketError(#[from] std::io::Error),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Authentication failed")]
    AuthFailed,
}
