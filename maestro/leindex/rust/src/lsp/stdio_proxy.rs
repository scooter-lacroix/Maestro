//! LSP Stdio Proxy
//!
//! Multiplexes multiple Unix socket clients to a single LSP process.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────┐  Unix Socket  ┌─────────────┐  stdio  ┌─────────┐
//! │ Client1 │ ─────────────▶│             │ ───────▶│         │
//! └─────────┘               │             │          │         │
//! ┌─────────┐               │  LspStdio   │          │   LSP   │
//! │ Client2 │ ─────────────▶│   Proxy     │ ───────▶│ Process │
//! └─────────┘               │             │          │         │
//! ┌─────────┘               │             │◀───────  │         │
//! │ ClientN │               └─────────────┘          └─────────┘
//! └─────────┘
//! ```
//!
//! ## Socket Path
//!
//! Each proxy creates a Unix socket at:
//! `/tmp/maestro-lsp-{language}-{session_id}.sock`
//!
//! ## Message Flow
//!
//! 1. Client connects → gets unique client_id
//! 2. Client sends request → proxy rewrites ID → forwards to LSP
//! 3. LSP responds → proxy routes back to original client
//! 4. LSP notifications → broadcast to all clients

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::ChildStdin;
use tokio::sync::{mpsc, watch, Mutex, RwLock};
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

use crate::memory::lsp_manager::LspType;

/// Maximum message size to prevent DoS (16MB)
const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Maximum header size for LSP messages
const MAX_HEADER_SIZE: usize = 1024;

/// Buffer size for socket I/O
const BUFFER_SIZE: usize = 16 * 1024;

/// Channel capacity per client
const CLIENT_CHANNEL_CAPACITY: usize = 256;

/// Current status of the LSP proxy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspProxyStatus {
    /// Proxy not started
    Stopped,
    /// Starting up (socket binding, process spawning)
    Starting,
    /// Running and accepting connections
    Running,
    /// Graceful shutdown in progress
    ShuttingDown,
    /// Error state
    Error,
}

impl std::fmt::Display for LspProxyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped => write!(f, "stopped"),
            Self::Starting => write!(f, "starting"),
            Self::Running => write!(f, "running"),
            Self::ShuttingDown => write!(f, "shutting_down"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// Tracks pending LSP requests for response routing
#[derive(Debug, Clone)]
struct PendingLspRequest {
    /// Client ID that made this request
    client_id: u64,
    /// Original request ID from client
    original_id: Value,
}

/// LSP stdio proxy that multiplexes multiple Unix socket clients to a single LSP process
pub struct LspStdioProxy {
    /// LSP type (Rust, Python, TypeScript)
    lsp_type: LspType,
    /// Session identifier (for socket path and logging)
    session_id: String,
    /// Unix socket path
    socket_path: PathBuf,
    /// Project path (working directory for LSP)
    project_path: String,
    /// Current proxy status
    status: Arc<RwLock<LspProxyStatus>>,
    /// Shutdown signal receiver
    shutdown_rx: watch::Receiver<bool>,
    /// Shutdown signal sender
    shutdown_tx: watch::Sender<bool>,
    /// Next unique client ID
    next_client_id: Arc<AtomicU64>,
}

impl LspStdioProxy {
    /// Create a new LSP stdio proxy
    ///
    /// ## Arguments
    ///
    /// - `lsp_type`: Type of LSP server to proxy
    /// - `session_id`: Session identifier for socket path
    /// - `project_path`: Working directory for LSP process
    pub fn new(lsp_type: LspType, session_id: &str, project_path: &str) -> Result<Self> {
        let socket_path = Self::socket_path_for(lsp_type, session_id);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Ok(Self {
            lsp_type,
            session_id: session_id.to_string(),
            socket_path,
            project_path: project_path.to_string(),
            status: Arc::new(RwLock::new(LspProxyStatus::Stopped)),
            shutdown_rx,
            shutdown_tx,
            next_client_id: Arc::new(AtomicU64::new(1)),
        })
    }

    /// Generate the socket path for this proxy
    ///
    /// ## Format
    ///
    /// `/tmp/maestro-lsp-{language}-{sanitized_session_id}.sock`
    ///
    /// Session ID is sanitized to alphanumeric + dash/underscore only
    pub fn socket_path_for(lsp_type: LspType, session_id: &str) -> PathBuf {
        let sanitized = session_id
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>();

        let filename = format!("maestro-lsp-{}-{}.sock", lsp_type.language(), sanitized);
        std::env::temp_dir().join(filename)
    }

    /// Start the proxy: bind socket, spawn LSP, begin accepting connections
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())` when proxy stops (shutdown or error)
    pub async fn run(&mut self) -> Result<()> {
        *self.status.write().await = LspProxyStatus::Starting;

        // Remove stale socket file if exists
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)
                .with_context(|| format!("Failed to remove stale socket file: {:?}", self.socket_path))?;
        }

        // Bind Unix socket
        let listener = UnixListener::bind(&self.socket_path)
            .with_context(|| format!("Failed to bind socket: {:?}", self.socket_path))?;

        info!(
            "LSP proxy listening on: {:?} (LSP: {})",
            self.socket_path,
            self.lsp_type.display_name()
        );

        // Spawn LSP process
        let mut lsp_command = tokio::process::Command::new(self.lsp_type.binary_name());
        lsp_command
            .current_dir(&self.project_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(true);

        // Add default arguments
        for arg in self.lsp_type.default_additional_args() {
            lsp_command.arg(arg);
        }

        let mut lsp_child = lsp_command.spawn()
            .with_context(|| format!("Failed to spawn LSP: {}", self.lsp_type.binary_name()))?;

        let lsp_stdin = lsp_child.stdin.take()
            .context("Failed to open LSP stdin")?;
        let lsp_stdout = lsp_child.stdout.take()
            .context("Failed to open LSP stdout")?;

        // Shared state
        let clients: Arc<Mutex<HashMap<u64, mpsc::Sender<Vec<u8>>>>> = Arc::new(Mutex::new(HashMap::new()));
        let pending: Arc<Mutex<HashMap<String, PendingLspRequest>>> = Arc::new(Mutex::new(HashMap::new()));
        let lsp_stdin_arc = Arc::new(Mutex::new(lsp_stdin));

        *self.status.write().await = LspProxyStatus::Running;

        // Clone shutdown receiver for the select loop
        let mut shutdown_rx = self.shutdown_rx.clone();

        // Spawn LSP stdout router task
        let lsp_stdout_task = {
            let clients_clone = Arc::clone(&clients);
            let pending_clone = Arc::clone(&pending);
            let status_clone = Arc::clone(&self.status);
            tokio::spawn(async move {
                if let Err(e) = Self::route_lsp_output(lsp_stdout, clients_clone, pending_clone).await {
                    error!("LSP stdout router error: {}", e);
                    *status_clone.write().await = LspProxyStatus::Error;
                }
            })
        };

        // Main accept loop
        loop {
            tokio::select! {
                // Check for shutdown signal
                Ok(()) = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("Shutdown signal received, stopping LSP proxy");
                        break;
                    }
                }

                // Accept new client connection
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((socket, addr)) => {
                            debug!("New client connected from: {:?}", addr);
                            let client_id = self.next_client_id.fetch_add(1, Ordering::SeqCst);

                            let lsp_stdin_clone = Arc::clone(&lsp_stdin_arc);
                            let clients_clone = Arc::clone(&clients);
                            let pending_clone = Arc::clone(&pending);
                            let status_clone = Arc::clone(&self.status);

                            tokio::spawn(async move {
                                if let Err(e) = Self::spawn_client(
                                    client_id,
                                    socket,
                                    lsp_stdin_clone,
                                    clients_clone,
                                    pending_clone,
                                    status_clone,
                                ).await {
                                    warn!("Client {} handler error: {}", client_id, e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept client connection: {}", e);
                        }
                    }
                }

                // Check if LSP process has exited
                _ = lsp_child.wait() => {
                    warn!("LSP process exited unexpectedly");
                    *self.status.write().await = LspProxyStatus::Error;
                    break;
                }
            }
        }

        // Cleanup
        lsp_stdout_task.abort();
        let _ = lsp_child.kill().await;
        let _ = std::fs::remove_file(&self.socket_path);

        *self.status.write().await = LspProxyStatus::Stopped;
        info!("LSP proxy stopped");

        Ok(())
    }

    /// Spawn a new client connection handler
    async fn spawn_client(
        client_id: u64,
        socket: UnixStream,
        lsp_stdin: Arc<Mutex<ChildStdin>>,
        clients: Arc<Mutex<HashMap<u64, mpsc::Sender<Vec<u8>>>>>,
        pending: Arc<Mutex<HashMap<String, PendingLspRequest>>>,
        _status: Arc<RwLock<LspProxyStatus>>,
    ) -> Result<()> {
        let (mut reader, mut writer) = socket.into_split();
        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(CLIENT_CHANNEL_CAPACITY);

        // Add client to shared map
        clients.lock().await.insert(client_id, tx.clone());

        // Spawn task to write messages from channel to socket
        let writer_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = writer.write_all(&msg).await {
                    warn!("Failed to write to client {}: {}", client_id, e);
                    break;
                }
            }
        });

        // Read messages from client and forward to LSP
        let mut buf_reader = BufReader::with_capacity(BUFFER_SIZE, reader);
        let mut line = String::new();

        loop {
            line.clear();
            match buf_reader.read_line(&mut line).await {
                Ok(0) => {
                    debug!("Client {} disconnected", client_id);
                    break;
                }
                Ok(_) => {
                    // Parse JSON-RPC from client
                    let json_str = line.trim();
                    if let Ok(request) = serde_json::from_str::<Value>(json_str) {
                        // Rewrite request ID to prevent collisions
                        if let Some(id) = request.get("id") {
                            let internal_id = format!("c{}-{}", client_id, id);
                            let mut modified = request.clone();
                            modified["id"] = Value::String(internal_id.clone());

                            // Store pending request
                            let pending_req = PendingLspRequest {
                                client_id,
                                original_id: id.clone(),
                            };
                            pending.lock().await.insert(internal_id.clone(), pending_req);

                            // Forward to LSP with Content-Length framing
                            let lsp_json = serde_json::to_string(&modified)?;
                            Self::write_lsp_message_to_stdin(&lsp_stdin, &lsp_json).await?;
                        } else {
                            // No ID - it's a notification, forward as-is
                            let lsp_json = serde_json::to_string(&request)?;
                            Self::write_lsp_message_to_stdin(&lsp_stdin, &lsp_json).await?;
                        }
                    }
                }
                Err(e) => {
                    warn!("Error reading from client {}: {}", client_id, e);
                    break;
                }
            }
        }

        // Cleanup
        writer_task.abort();
        clients.lock().await.remove(&client_id);

        Ok(())
    }

    /// Read LSP stdout and route responses to clients
    async fn route_lsp_output(
        mut lsp_stdout: tokio::process::ChildStdout,
        clients: Arc<Mutex<HashMap<u64, mpsc::Sender<Vec<u8>>>>>,
        pending: Arc<Mutex<HashMap<String, PendingLspRequest>>>,
    ) -> Result<()> {
        loop {
            match Self::read_next_lsp_message(&mut lsp_stdout).await {
                Ok(message) => {
                    // Check if this is a notification (no "id" field)
                    if let Some(id_value) = message.get("id") {
                        // This is a response - route to specific client
                        if let Some(id_str) = id_value.as_str() {
                            let pending_guard = pending.lock().await;
                            if let Some(pending_req) = pending_guard.get(id_str) {
                                let mut response = message.clone();
                                response["id"] = pending_req.original_id.clone();

                                let clients_guard = clients.lock().await;
                                if let Some(tx) = clients_guard.get(&pending_req.client_id) {
                                    let response_json = format!("{}\n", serde_json::to_string(&response)?);
                                    let _ = tx.send(response_json.into_bytes()).await;
                                }
                            }
                        }
                    } else {
                        // This is a notification - broadcast to all clients
                        let notification_json = format!("{}\n", serde_json::to_string(&message)?);
                        let clients_guard = clients.lock().await;
                        for tx in clients_guard.values() {
                            let _ = tx.send(notification_json.clone().into_bytes()).await;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read from LSP stdout: {}", e);
                    return Err(e);
                }
            }
        }
    }

    /// Write an LSP message to stdin
    async fn write_lsp_message_to_stdin(
        lsp_stdin: &Arc<Mutex<ChildStdin>>,
        message: &str,
    ) -> Result<()> {
        let header = format!("Content-Length: {}\r\n\r\n", message.len());
        let mut stdin = lsp_stdin.lock().await;
        stdin.write_all(header.as_bytes()).await?;
        stdin.write_all(message.as_bytes()).await?;
        stdin.flush().await?;
        Ok(())
    }

    /// Read the next LSP message from stdout
    async fn read_next_lsp_message(stdout: &mut tokio::process::ChildStdout) -> Result<Value> {
        // Read headers
        let mut content_length = None;
        let mut header_buf = Vec::with_capacity(256);

        loop {
            let ch = stdout.read_u8().await?;
            header_buf.push(ch);

            if header_buf.len() >= 4 {
                let last4 = &header_buf[header_buf.len() - 4..];
                if last4 == b"\r\n\r\n" {
                    break;
                }
            }

            if header_buf.len() >= MAX_HEADER_SIZE {
                return Err(anyhow!("LSP headers too large"));
            }
        }

        let headers = std::str::from_utf8(&header_buf)?;
        for line in headers.lines() {
            if line.to_lowercase().starts_with("content-length:") {
                let len_str = line.split(':').nth(1).unwrap_or("").trim();
                content_length = Some(len_str.parse::<usize>()?);
            }
        }

        let length = content_length.ok_or_else(|| anyhow!("Missing Content-Length header"))?;

        if length > MAX_MESSAGE_SIZE {
            return Err(anyhow!("LSP message too large: {} bytes", length));
        }

        // Read body
        let mut buffer = vec![0u8; length];
        stdout.read_exact(&mut buffer).await?;

        let content = String::from_utf8(buffer)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Signal graceful shutdown
    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    /// Check if proxy is currently running
    pub async fn is_running(&self) -> bool {
        *self.status.read().await == LspProxyStatus::Running
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the LSP type
    pub fn lsp_type(&self) -> LspType {
        self.lsp_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path_generation() {
        let path = LspStdioProxy::socket_path_for(LspType::Rust, "session-123");
        assert!(path.to_string_lossy().contains("maestro-lsp-rust"));
        assert!(path.to_string_lossy().contains("session-123"));
        assert!(path.extension().unwrap() == "sock");
    }

    #[test]
    fn test_socket_path_sanitization() {
        // Special characters should be replaced with underscores
        let path = LspStdioProxy::socket_path_for(LspType::Python, "session@test/123");
        let path_str = path.to_string_lossy();
        // Check that session-specific special chars are sanitized
        assert!(path_str.contains("session_test_123") || path_str.contains("session@test_123"));
        // The @ should be replaced
        assert!(!path_str.contains("session@test/123"));
    }

    #[test]
    fn test_proxy_status_display() {
        assert_eq!(LspProxyStatus::Running.to_string(), "running");
        assert_eq!(LspProxyStatus::Stopped.to_string(), "stopped");
        assert_eq!(LspProxyStatus::Error.to_string(), "error");
    }

    #[test]
    fn test_proxy_creation() {
        let proxy = LspStdioProxy::new(LspType::TypeScript, "test-session", "/tmp");
        assert!(proxy.is_ok());
        let proxy = proxy.unwrap();
        assert_eq!(proxy.session_id(), "test-session");
        assert_eq!(proxy.lsp_type(), LspType::TypeScript);
    }
}
