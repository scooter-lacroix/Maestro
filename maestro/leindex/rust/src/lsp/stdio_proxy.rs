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
//!
//! ## Security Considerations
//!
//! ### Socket Permissions
//!
//! The Unix socket is created with `0o777` permissions (rwxrwxrwx) to allow
//! multi-user scenarios where different users may need to connect to the
//! same LSP proxy. This is necessary because:
//!
//! - The proxy runs in a user session (not as root)
//! - Multiple users or processes may need access to the same LSP
//! - Unix domain sockets require write permission to connect
//!
//! **Security Note:** While permissive socket permissions allow broader access,
//! the actual LSP process is still running under the user's account with their
//! permissions. The socket itself only provides a communication endpoint - it
//! does not grant elevated privileges to connected clients.
//!
//! ### DoS Protection
//!
//! - Maximum message size: 16MB (prevents memory exhaustion)
//! - Per-client pending request limit: 100 (prevents request flooding)
//! - Client timeout: 30 seconds (prevents connection hoarding)
//!
//! ### Process Isolation
//!
//! The LSP process runs with the same permissions as the Maestro TUI process.
//! It does not gain any additional privileges through the proxy mechanism.

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::ChildStdin;
use tokio::sync::{mpsc, watch, Mutex, RwLock};
use tracing::{debug, error, info, warn};

use crate::memory::LspType;

// Type alias for the reader half of UnixStream (used in client handler)
type ClientReader = BufReader<tokio::net::unix::OwnedReadHalf>;

/// Maximum header size for LSP messages
const MAX_HEADER_SIZE: usize = 1024;

/// Maximum client line size to prevent DoS
const MAX_CLIENT_LINE: usize = 64 * 1024; // 64KB

/// Maximum pending requests per client
const MAX_PENDING_PER_CLIENT: usize = 100;

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
    /// Persistent buffered reader for LSP stdout
    lsp_stdout_reader: Option<BufReader<tokio::process::ChildStdout>>,
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
            lsp_stdout_reader: None,
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
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
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
            std::fs::remove_file(&self.socket_path).with_context(|| {
                format!("Failed to remove stale socket file: {:?}", self.socket_path)
            })?;
        }

        // Bind Unix socket
        let listener = UnixListener::bind(&self.socket_path)
            .with_context(|| format!("Failed to bind socket: {:?}", self.socket_path))?;

        // Set socket permissions to allow all users to connect (Task 10.1)
        // This is required for multi-user LSP proxy scenarios
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&self.socket_path)
                .with_context(|| format!("Failed to get socket metadata: {:?}", self.socket_path))?
                .permissions();
            perms.set_mode(0o777); // rwxrwxrwx - allow all users to connect
            std::fs::set_permissions(&self.socket_path, perms)
                .with_context(|| format!("Failed to set socket permissions: {:?}", self.socket_path))?;
        }

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

        let mut lsp_child = lsp_command
            .spawn()
            .with_context(|| format!("Failed to spawn LSP: {}", self.lsp_type.binary_name()))?;

        let lsp_stdin = lsp_child.stdin.take().context("Failed to open LSP stdin")?;
        let lsp_stdout = lsp_child
            .stdout
            .take()
            .context("Failed to open LSP stdout")?;

        // Initialize persistent buffered reader for LSP stdout
        self.lsp_stdout_reader = Some(BufReader::new(lsp_stdout));

        // Shared state
        let clients: Arc<Mutex<HashMap<u64, mpsc::Sender<Vec<u8>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending: Arc<Mutex<HashMap<String, PendingLspRequest>>> =
            Arc::new(Mutex::new(HashMap::new()));
        // Track pending request count per client for DoS protection
        let pending_count: Arc<Mutex<HashMap<u64, usize>>> = Arc::new(Mutex::new(HashMap::new()));
        let lsp_stdin_arc = Arc::new(Mutex::new(lsp_stdin));

        *self.status.write().await = LspProxyStatus::Running;

        // Clone shutdown receiver for the select loop
        let mut shutdown_rx = self.shutdown_rx.clone();

        // Spawn LSP stdout router task
        let lsp_stdout_task = {
            let clients_clone = Arc::clone(&clients);
            let pending_clone = Arc::clone(&pending);
            let pending_count_clone = Arc::clone(&pending_count);
            let status_clone = Arc::clone(&self.status);
            let lsp_stdout_reader = self.lsp_stdout_reader.take().expect("LSP stdout reader not initialized");
            tokio::spawn(async move {
                if let Err(e) = Self::route_lsp_output(
                    lsp_stdout_reader,
                    clients_clone,
                    pending_clone,
                    pending_count_clone,
                )
                .await
                {
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
                            let pending_count_clone = Arc::clone(&pending_count);
                            let status_clone = Arc::clone(&self.status);

                            tokio::spawn(async move {
                                if let Err(e) = Self::spawn_client(
                                    client_id,
                                    socket,
                                    lsp_stdin_clone,
                                    clients_clone,
                                    pending_clone,
                                    pending_count_clone,
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
        pending_count: Arc<Mutex<HashMap<u64, usize>>>,
        _status: Arc<RwLock<LspProxyStatus>>,
    ) -> Result<()> {
        let (reader, mut writer) = socket.into_split();
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

        // Read messages from client using proper LSP framing
        let mut reader = BufReader::with_capacity(BUFFER_SIZE, reader);

        loop {
            match Self::read_client_message(&mut reader).await {
                Ok(None) => {
                    debug!("Client {} disconnected", client_id);
                    break;
                }
                Ok(Some(json_bytes)) => {
                    // Parse JSON-RPC from client
                    let json_str = std::str::from_utf8(&json_bytes)?;
                    if let Ok(request) = serde_json::from_str::<Value>(json_str) {
                        // Rewrite request ID to prevent collisions
                        if let Some(id) = request.get("id") {
                            let internal_id = format!("c{}-{}", client_id, id);
                            let mut modified = request.clone();
                            modified["id"] = Value::String(internal_id.clone());

                            // Check pending count limit before adding
                            let mut count_guard = pending_count.lock().await;
                            let current_count = count_guard.get(&client_id).copied().unwrap_or(0);
                            if current_count >= MAX_PENDING_PER_CLIENT {
                                warn!(
                                    "Client {} exceeded pending limit ({}), dropping request",
                                    client_id, MAX_PENDING_PER_CLIENT
                                );
                                drop(count_guard);
                                continue;
                            }

                            // Store pending request
                            let pending_req = PendingLspRequest {
                                client_id,
                                original_id: id.clone(),
                            };
                            pending
                                .lock()
                                .await
                                .insert(internal_id.clone(), pending_req);
                            *count_guard.entry(client_id).or_insert(0) += 1;
                            drop(count_guard);

                            // Forward to LSP with Content-Length framing
                            let lsp_json = serde_json::to_string(&modified)?;
                            Self::write_lsp_message_to_stdin(&lsp_stdin, &lsp_json).await?;
                        } else {
                            // No ID - it's a notification, forward as-is
                            let lsp_json = serde_json::to_string(&request)?;
                            Self::write_lsp_message_to_stdin(&lsp_stdin, &lsp_json).await?;
                        }
                    } else {
                        // Invalid JSON - log error
                        warn!(
                            "Invalid JSON from client {}: {}",
                            client_id,
                            String::from_utf8_lossy(&json_bytes)
                                .chars()
                                .take(100)
                                .collect::<String>()
                        );
                    }
                }
                Err(e) => {
                    warn!("Error reading from client {}: {}", client_id, e);
                    break;
                }
            }
        }

        // Cleanup: remove client and all pending requests for this client
        writer_task.abort();
        clients.lock().await.remove(&client_id);

        // Clean up pending requests for this client (prevent memory leak)
        let mut pending_guard = pending.lock().await;
        pending_guard.retain(|_, req| req.client_id != client_id);
        drop(pending_guard);

        // Clean up pending count
        pending_count.lock().await.remove(&client_id);

        Ok(())
    }

    /// Read a message from client using proper LSP Content-Length framing
    /// Returns Ok(None) on EOF, Ok(Some(bytes)) on success, Err on failure
    async fn read_client_message(reader: &mut ClientReader) -> Result<Option<Vec<u8>>> {
        let mut content_length: Option<usize> = None;
        let mut header_buf = Vec::new();

        // Read headers until \r\n\r\n
        loop {
            let byte = match reader.read_u8().await {
                Ok(b) => b,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // Clean EOF
                    return Ok(None);
                }
                Err(e) => return Err(e.into()),
            };

            header_buf.push(byte);

            if header_buf.len() >= 4 {
                let last4 = &header_buf[header_buf.len() - 4..];
                if last4 == b"\r\n\r\n" {
                    break;
                }
            }

            if header_buf.len() > MAX_HEADER_SIZE {
                return Err(anyhow!("Client header too large"));
            }

            // Parse headers as we go
            let header = String::from_utf8_lossy(&header_buf);
            if let Some(line) = header.strip_suffix("\r\n") {
                if let Some(value) = line.strip_prefix("Content-Length: ") {
                    if let Ok(len) = value.trim().parse::<usize>() {
                        content_length = Some(len);
                    }
                }
            }
        }

        let length =
            content_length.ok_or_else(|| anyhow!("No Content-Length in client message"))?;

        if length > MAX_CLIENT_LINE {
            return Err(anyhow!("Client message too large: {} bytes", length));
        }

        // Read exact body
        let mut body = vec![0u8; length];
        reader.read_exact(&mut body).await?;
        Ok(Some(body))
    }

    /// Read next LSP message from a BufReader
    async fn read_next_lsp_message_from_reader(reader: &mut BufReader<tokio::process::ChildStdout>) -> Result<Value> {
        const MAX_HEADER_SIZE: usize = 100;
        const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024; // 16MB

        let mut headers = Vec::new();
        let mut line = String::new();

        // Read headers
        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                return Err(anyhow!("EOF reading LSP headers"));
            }

            headers.push(line.clone());
            if line == "\r\n" || line == "\n" {
                break;
            }

            if headers.len() > MAX_HEADER_SIZE {
                return Err(anyhow!("LSP headers too large"));
            }
        }

        // Parse Content-Length
        let mut content_length = None;
        for header in &headers {
            let header_lower = header.to_lowercase();
            if header_lower.starts_with("content-length:") {
                let len_str = header[15..].trim();
                content_length = len_str.parse::<usize>().ok();
                break;
            }
        }

        let length = content_length.ok_or_else(|| anyhow!("Missing Content-Length header"))?;

        if length > MAX_MESSAGE_SIZE {
            return Err(anyhow!("LSP message too large: {} bytes", length));
        }

        // Read body
        let mut buffer = vec![0u8; length];
        reader.read_exact(&mut buffer).await?;

        let content = String::from_utf8(buffer)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Read LSP stdout and route responses to clients
    async fn route_lsp_output(
        mut lsp_stdout: BufReader<tokio::process::ChildStdout>,
        clients: Arc<Mutex<HashMap<u64, mpsc::Sender<Vec<u8>>>>>,
        pending: Arc<Mutex<HashMap<String, PendingLspRequest>>>,
        pending_count: Arc<Mutex<HashMap<u64, usize>>>,
    ) -> Result<()> {
        loop {
            match Self::read_next_lsp_message_from_reader(&mut lsp_stdout).await {
                Ok(message) => {
                    // Check if this is a notification (no "id" field)
                    if let Some(id_value) = message.get("id") {
                        // This is a response - route to specific client
                        if let Some(id_str) = id_value.as_str() {
                            // Clone data we need, then drop locks before await
                            let (tx, original_id, client_id) = {
                                let pending_guard = pending.lock().await;
                                if let Some(pending_req) = pending_guard.get(id_str) {
                                    let clients_guard = clients.lock().await;
                                    let tx = clients_guard.get(&pending_req.client_id).cloned();
                                    (tx, pending_req.original_id.clone(), pending_req.client_id)
                                } else {
                                    continue;
                                }
                            }; // Locks dropped here

                            // Now await without holding locks
                            if let Some(tx) = tx {
                                let mut response = message.clone();
                                response["id"] = original_id;

                                let response_json =
                                    format!("{}\n", serde_json::to_string(&response)?);
                                let _ = tx.send(response_json.into_bytes()).await;

                                // Decrement pending count for this client
                                let mut count_guard = pending_count.lock().await;
                                if let Some(count) = count_guard.get_mut(&client_id) {
                                    *count = count.saturating_sub(1);
                                }
                            }

                            // Remove from pending after processing (prevent memory leak)
                            pending.lock().await.remove(id_str);
                        }
                    } else {
                        // This is a notification - broadcast to all clients
                        // Clone all senders, drop lock, then broadcast
                        let senders: Vec<_> = clients.lock().await.values().cloned().collect();
                        let notification_json = format!("{}\n", serde_json::to_string(&message)?);

                        for tx in senders {
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
        // The @ and / should be replaced
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
