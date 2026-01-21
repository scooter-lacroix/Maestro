//! LSP Manager
//!
//! Manages Language Server Protocol (LSP) server lifecycle for on-demand code intelligence.
//!
//! ## Architecture
//!
//! The LSP manager handles:
//! - **Process Spawning**: Starting LSP server processes (rust-analyzer, ruff-lsp, typescript-language-server)
//! - **Lifecycle Management**: Starting, stopping, monitoring LSP processes
//! - **State Persistence**: Storing LSP state in Turso database
//! - **Language Detection**: Auto-starting appropriate LSPs based on project files
//! - **Graceful Degradation**: Continuing operation even when LSPs fail
//!
//! ## Supported LSPs
//!
//! - **rust-analyzer**: Rust language server
//! - **ruff-lsp**: Python language server
//! - **typescript-language-server**: TypeScript/JavaScript language server
//!
//! ## Usage
//!
//! ```no_run
//! use leindex_analyzers::memory::{LspManager, LspType, TursoStorageBackend};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let storage = TursoStorageBackend::in_memory(None).await?;
//!     let manager = LspManager::new(storage);
//!     // Start LSP for a session
//!     manager.start_lsp("session-123", LspType::Rust, None).await?;
//!     Ok(())
//! }
//! ```

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Child as TokioChild;
use tokio::process::Command as TokioCommand;
use tokio::sync::{watch, RwLock};
use tokio::task::JoinHandle;
use tokio::time::{interval, timeout};
use tracing::{debug, info, warn};

use super::turso_backend::{LspServerState, LspStatus, TursoStorageBackend};
use crate::lsp::stdio_proxy::LspStdioProxy;

#[cfg(unix)]
/// LSP server types supported by Maestro
///
/// Each variant represents a specific language server that can be spawned on-demand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LspType {
    /// rust-analyzer - Rust language server
    Rust,
    /// ruff-lsp - Python language server
    Python,
    /// typescript-language-server - TypeScript/JavaScript language server
    TypeScript,
}

impl LspType {
    /// Get the binary name for this LSP
    pub fn binary_name(&self) -> &'static str {
        match self {
            LspType::Rust => "rust-analyzer",
            LspType::Python => "ruff-lsp",
            LspType::TypeScript => "typescript-language-server",
        }
    }

    /// Get the display name for this LSP
    pub fn display_name(&self) -> &'static str {
        match self {
            LspType::Rust => "rust-analyzer",
            LspType::Python => "ruff-lsp",
            LspType::TypeScript => "typescript-language-server",
        }
    }

    /// Get file extensions that trigger this LSP
    pub fn file_extensions(&self) -> &'static [&'static str] {
        match self {
            LspType::Rust => &["rs"],
            LspType::Python => &["py"],
            LspType::TypeScript => &["ts", "tsx", "js", "jsx"],
        }
    }

    /// Get the language name for this LSP
    pub fn language(&self) -> &'static str {
        match self {
            LspType::Rust => "rust",
            LspType::Python => "python",
            LspType::TypeScript => "typescript",
        }
    }

    pub fn default_additional_args(&self) -> &'static [&'static str] {
        match self {
            // typescript-language-server defaults to TCP unless --stdio is provided.
            LspType::TypeScript => &["--stdio"],
            _ => &[],
        }
    }
}

/// LSP server configuration
///
/// Configuration for LSP server behavior and lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspConfig {
    /// Whether to auto-start this LSP when language is detected
    pub auto_start: bool,
    /// Custom path to LSP binary (if not in PATH)
    pub binary_path: Option<PathBuf>,
    /// Additional arguments to pass to LSP
    pub additional_args: Vec<String>,
    /// Environment variables for LSP process
    pub env_vars: HashMap<String, String>,
    /// Whether to use stdio proxy for this LSP (requires feature flag)
    #[serde(default)]
    pub use_proxy: bool,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            auto_start: true,
            binary_path: None,
            additional_args: Vec::new(),
            env_vars: HashMap::new(),
            use_proxy: false,
        }
    }
}

/// LSP process tracking information
///
/// Tracks runtime information about a running LSP server process.
pub struct LspProcess {
    /// LSP type
    pub lsp_type: LspType,
    /// Session ID this LSP belongs to
    pub session_id: String,
    /// Process ID (if running)
    pub pid: Option<u32>,
    /// Current status
    pub status: LspStatus,
    /// Port (if using TCP communication)
    pub port: Option<u16>,
    /// Whether to auto-start this LSP when language is detected
    pub auto_start: bool,
    /// When the process was started
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last error message (if any)
    pub last_error: Option<String>,
    /// Child process handle (kept for cleanup)
    child: Option<TokioChild>,
    /// Process group ID (Unix only) used for killing child sub-processes.
    pgid: Option<i32>,
    /// Stdio proxy task handle (if proxy is enabled)
    proxy_task: Option<tokio::task::JoinHandle<()>>,
    /// Stdio proxy socket path (if proxy is enabled)
    proxy_socket_path: Option<PathBuf>,
    /// Whether stdio proxy is enabled for this LSP
    use_proxy: bool,
}

// Implement Clone manually since TokioChild doesn't support Clone
impl Clone for LspProcess {
    fn clone(&self) -> Self {
        Self {
            lsp_type: self.lsp_type,
            session_id: self.session_id.clone(),
            pid: self.pid,
            status: self.status,
            port: self.port,
            auto_start: self.auto_start,
            started_at: self.started_at,
            last_error: self.last_error.clone(),
            child: None, // Cannot clone the child handle
            pgid: self.pgid,
            proxy_task: None, // Cannot clone the task handle
            proxy_socket_path: self.proxy_socket_path.clone(),
            use_proxy: self.use_proxy,
        }
    }
}

impl LspProcess {
    /// Create a new LSP process tracking record
    pub fn new(lsp_type: LspType, session_id: String, use_proxy: bool) -> Self {
        Self {
            lsp_type,
            session_id,
            pid: None,
            status: LspStatus::Stopped,
            port: None,
            auto_start: true,
            started_at: None,
            last_error: None,
            child: None,
            pgid: None,
            proxy_task: None,
            proxy_socket_path: None,
            use_proxy,
        }
    }

    /// Kill the underlying LSP process
    ///
    /// Maestro currently does not implement the LSP `shutdown`/`exit` JSON-RPC handshake, so
    /// termination is OS-signal based. On Unix we signal the entire process group to avoid
    /// leaking child processes.
    pub async fn kill(&mut self) -> Result<()> {
        self.kill_with_timeout(Duration::from_secs(5)).await
    }

    pub(crate) async fn kill_with_timeout(&mut self, max_wait: Duration) -> Result<()> {
        let pid = self.pid;
        let pgid = self.pgid;

        // First, shut down the stdio proxy if it's running
        if let Some(proxy_task) = self.proxy_task.take() {
            debug!(
                "Aborting stdio proxy task for LSP '{}'",
                self.lsp_type.display_name()
            );
            proxy_task.abort();

            // Clean up socket file if it exists
            if let Some(ref socket_path) = self.proxy_socket_path {
                if let Err(e) = std::fs::remove_file(socket_path) {
                    // Log error but don't fail - the file might already be gone
                    debug!(
                        "Failed to remove proxy socket file {:?}: {}",
                        socket_path, e
                    );
                }
            }

            self.proxy_socket_path = None;
        }

        if let Some(mut child) = self.child.take() {
            debug!("Stopping LSP process (PID: {:?}, PGID: {:?})", pid, pgid);

            #[cfg(unix)]
            {
                if let Some(pgid) = pgid {
                    // Try SIGTERM first.
                    if let Err(e) = unix_kill_process_group(pgid, libc::SIGTERM) {
                        warn!(
                            "Failed to SIGTERM LSP process group (PGID: {}): {}",
                            pgid, e
                        );
                    }

                    match timeout(max_wait, child.wait()).await {
                        Ok(Ok(exit_status)) => {
                            debug!("LSP process exited after SIGTERM: {:?}", exit_status);
                        }
                        Ok(Err(e)) => {
                            self.child = Some(child);
                            return Err(anyhow!("LSP process wait failed: {}", e));
                        }
                        Err(_) => {
                            debug!(
                                "LSP process did not exit within {:?}; sending SIGKILL to process group",
                                max_wait
                            );
                            if let Err(e) = unix_kill_process_group(pgid, libc::SIGKILL) {
                                warn!(
                                    "Failed to SIGKILL LSP process group (PGID: {}): {}",
                                    pgid, e
                                );
                            }
                            match timeout(max_wait, child.wait()).await {
                                Ok(Ok(exit_status)) => {
                                    debug!("LSP process exited after SIGKILL: {:?}", exit_status);
                                }
                                Ok(Err(e)) => {
                                    self.child = Some(child);
                                    return Err(anyhow!(
                                        "LSP process wait failed after SIGKILL: {}",
                                        e
                                    ));
                                }
                                Err(_) => {
                                    self.child = Some(child);
                                    return Err(anyhow!(
                                        "Timed out waiting for LSP process to exit after SIGKILL (PID: {:?}, PGID: {:?})",
                                        pid,
                                        pgid
                                    ));
                                }
                            }
                        }
                    }
                } else {
                    // Fallback: kill just the direct child.
                    let _ = child.kill().await;
                    match timeout(max_wait, child.wait()).await {
                        Ok(Ok(_)) => {}
                        Ok(Err(e)) => {
                            self.child = Some(child);
                            return Err(anyhow!("LSP process wait failed: {}", e));
                        }
                        Err(_) => {
                            self.child = Some(child);
                            return Err(anyhow!(
                                "Timed out waiting for LSP process to exit (PID: {:?})",
                                pid
                            ));
                        }
                    }
                }
            }

            #[cfg(not(unix))]
            {
                let _ = child.kill().await;
                match timeout(max_wait, child.wait()).await {
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => {
                        self.child = Some(child);
                        return Err(anyhow!("LSP process wait failed: {}", e));
                    }
                    Err(_) => {
                        self.child = Some(child);
                        return Err(anyhow!(
                            "Timed out waiting for LSP process to exit (PID: {:?})",
                            pid
                        ));
                    }
                }
            }
        } else {
            // No child handle, but on Unix we may still have a process group to clean up.
            #[cfg(unix)]
            if let Some(pgid) = pgid {
                if let Err(e) = unix_kill_process_group(pgid, libc::SIGKILL) {
                    warn!(
                        "Failed to SIGKILL LSP process group (PGID: {}): {}",
                        pgid, e
                    );
                }
            }
        }

        self.status = LspStatus::Stopped;
        self.pid = None;
        self.pgid = None;
        Ok(())
    }

    /// Convert to database state for persistence
    pub fn to_db_state(&self) -> LspServerState {
        LspServerState {
            id: 0, // Will be set by database
            session_id: self.session_id.clone(),
            language: self.lsp_type.language().to_string(),
            lsp_name: self.lsp_type.binary_name().to_string(),
            status: self.status,
            pid: self.pid.map(|p| p as i64),
            port: self.port.map(|p| p as i64),
            auto_start: self.auto_start,
            last_started: self.started_at.map(|d| d.to_rfc3339()),
            last_error: self.last_error.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// Check if the underlying process is still running
    ///
    /// Returns Some(true) if running, Some(false) if terminated, None if no child process
    pub async fn is_alive(&mut self) -> Option<bool> {
        if let Some(mut child) = self.child.take() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    debug!(
                        "LSP process (PID: {:?}) exited with status: {:?}",
                        self.pid, status
                    );
                    self.status = LspStatus::Error;
                    self.pid = None;
                    self.last_error = Some(format!(
                        "Process exited unexpectedly with status: {:?}",
                        status
                    ));
                    // Keep self.pgid for possible process-group cleanup.
                    Some(false)
                }
                Ok(None) => {
                    self.child = Some(child);
                    Some(true)
                }
                Err(e) => {
                    warn!("Error checking LSP process (PID: {:?}): {}", self.pid, e);
                    self.status = LspStatus::Error;
                    self.pid = None;
                    self.last_error = Some(format!("Error checking process: {}", e));
                    // Keep self.pgid for possible process-group cleanup.
                    Some(false)
                }
            }
        } else {
            // No child process to check
            None
        }
    }
}

fn spawn_output_drain<R>(mut reader: R, label: String)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(_) => {}
                Err(e) => {
                    debug!("Stopped draining {} due to read error: {}", label, e);
                    break;
                }
            }
        }
    });
}

#[cfg(unix)]
fn unix_kill_process_group(pgid: i32, signal: i32) -> std::io::Result<()> {
    // Negative pid => process group.
    let res = unsafe { libc::kill(-pgid, signal) };
    if res == 0 {
        return Ok(());
    }
    let err = std::io::Error::last_os_error();
    // ESRCH means nothing left to kill.
    if err.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    Err(err)
}

/// Get the process group ID for a process
#[cfg(unix)]
fn get_process_group_id(pid: Option<u32>) -> Option<i32> {
    pid.and_then(|p| unsafe {
        let pgid = libc::getpgid(p as i32);
        if pgid < 0 {
            None
        } else {
            Some(pgid)
        }
    })
}

/// LSP Manager
///
/// Manages the lifecycle of LSP server processes for code intelligence.
///
/// ## Responsibilities
///
/// - **Spawn**: Start LSP server processes on-demand
/// - **Monitor**: Track LSP process health and status
/// - **Persist**: Store LSP state in Turso database
/// - **Cleanup**: Stop LSP processes when sessions end
///
/// ## Graceful Degradation
///
/// The manager follows graceful degradation principles:
/// - If an LSP fails to start, log the error but continue
/// - If an LSP crashes during operation, log and don't block other operations
/// - If database persistence fails, continue with in-memory tracking
#[derive(Clone)]
pub struct LspManager {
    /// Storage backend for persisting LSP state
    storage: TursoStorageBackend,
    /// Running LSP processes (session_id + lsp_name -> process info)
    running_lsps: Arc<RwLock<HashMap<String, LspProcess>>>,
    /// Running MCP bridge processes (session_id + lsp_name -> child handle)
    running_bridges: Arc<RwLock<HashMap<String, TokioChild>>>,
    /// Cancellation flag for background monitoring tasks started via `start_monitoring`.
    monitor_stop_tx: watch::Sender<bool>,
}

// Use std::sync::Arc for tokio compatibility
use std::sync::Arc;

impl LspManager {
    /// Create a new LSP manager
    ///
    /// ## Arguments
    ///
    /// - `storage`: Turso storage backend for state persistence
    ///
    /// ## Returns
    ///
    /// Returns a new `LspManager` instance
    pub fn new(storage: TursoStorageBackend) -> Self {
        info!("Creating LSP manager");
        let (monitor_stop_tx, _monitor_stop_rx) = watch::channel(false);
        Self {
            storage,
            running_lsps: Arc::new(RwLock::new(HashMap::new())),
            running_bridges: Arc::new(RwLock::new(HashMap::new())),
            monitor_stop_tx,
        }
    }

    /// Create LSP manager with default storage
    ///
    /// Uses default Turso database location.
    pub async fn with_default_storage() -> Result<Self> {
        let storage = TursoStorageBackend::new(None, None).await?;
        let manager = Self::new(storage);
        if let Err(e) = manager.restore_lsps_on_startup().await {
            warn!("Failed to restore LSPs on startup: {}", e);
        }
        Ok(manager)
    }

    /// Start an LSP server for a session
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to start
    /// - `config`: Optional LSP configuration
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())` if the LSP started successfully, or an error with details.
    ///
    /// The manager still updates in-memory state to `Error` (and best-effort persists it) so
    /// callers can choose to degrade gracefully (log + continue) without losing observability.
    pub async fn start_lsp(
        &self,
        session_id: &str,
        lsp_type: LspType,
        config: Option<LspConfig>,
    ) -> Result<()> {
        let config = config.unwrap_or_default();
        let lsp_key = format!("{}:{}", session_id, lsp_type.binary_name());

        info!(
            "Starting LSP '{}' for session '{}'",
            lsp_type.display_name(),
            session_id
        );

        // Check if already running
        {
            let running = self.running_lsps.read().await;
            if let Some(existing) = running.get(&lsp_key) {
                if existing.status == LspStatus::Running {
                    debug!(
                        "LSP '{}' already running for session '{}'",
                        lsp_type.display_name(),
                        session_id
                    );
                    return Ok(());
                }
            }
        }

        // Update status to starting
        let use_proxy = config.use_proxy;
        {
            let mut running = self.running_lsps.write().await;
            let mut process = LspProcess::new(lsp_type, session_id.to_string(), use_proxy);
            process.status = LspStatus::Starting;
            process.auto_start = config.auto_start;
            running.insert(lsp_key.clone(), process);
        }

        // Spawn the LSP process (unless using proxy, which will spawn it)
        let binary = config
            .binary_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(lsp_type.binary_name()));

        debug!("Spawning LSP process: {:?}", binary);

        let mut args: Vec<String> = config.additional_args;
        for default_arg in lsp_type.default_additional_args() {
            if !args.iter().any(|a| a == default_arg) {
                args.push((*default_arg).to_string());
            }
        }

        let mut cmd = TokioCommand::new(&binary);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&config.env_vars)
            .kill_on_drop(true);

        #[cfg(unix)]
        unsafe {
            cmd.pre_exec(|| {
                // Create a new process group so we can kill the entire LSP tree on shutdown.
                if libc::setpgid(0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        let result = cmd.spawn();

        match result {
            Ok(mut child) => {
                let pid = child.id();
                info!(
                    "LSP '{}' started successfully (PID: {})",
                    lsp_type.display_name(),
                    pid.unwrap_or(0)
                );

                // Drain stdout/stderr in the background so the child can't block on full pipes.
                if let Some(stdout) = child.stdout.take() {
                    spawn_output_drain(
                        stdout,
                        format!("{}:{} stdout", session_id, lsp_type.binary_name()),
                    );
                }
                if let Some(stderr) = child.stderr.take() {
                    spawn_output_drain(
                        stderr,
                        format!("{}:{} stderr", session_id, lsp_type.binary_name()),
                    );
                }

                // Update status to running and store child handle
                {
                    let mut running = self.running_lsps.write().await;
                    let process = running.get_mut(&lsp_key).unwrap();
                    process.pid = pid;
                    #[cfg(unix)]
                    {
                        process.pgid = pid.map(|p| p as i32);
                    }
                    process.status = LspStatus::Running;
                    process.started_at = Some(chrono::Utc::now());
                    process.child = Some(child);
                }

                // Spawn stdio proxy if enabled
                if use_proxy {
                    let project_path =
                        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

                    match LspStdioProxy::new(
                        lsp_type,
                        session_id,
                        project_path.to_string_lossy().as_ref(),
                    ) {
                        Ok(mut proxy) => {
                            let socket_path = proxy.socket_path().clone();
                            let socket_path_for_log = socket_path.clone();

                            // Spawn proxy in background task
                            let handle = match tokio::runtime::Handle::try_current() {
                                Ok(handle) => handle,
                                Err(_) => {
                                    warn!("No tokio runtime found for stdio proxy, continuing without proxy");
                                    let mut running = self.running_lsps.write().await;
                                    if let Some(process) = running.get_mut(&lsp_key) {
                                        process.use_proxy = false;
                                    }
                                    drop(running);
                                    // Persist to database
                                    if let Err(e) =
                                        self.persist_lsp_state(session_id, lsp_type).await
                                    {
                                        warn!("Failed to persist LSP state to database: {}", e);
                                    }
                                    return Ok(());
                                }
                            };

                            let proxy_task = handle.spawn(async move {
                                if let Err(e) = proxy.run().await {
                                    warn!("LSP stdio proxy exited with error: {}", e);
                                }
                            });

                            // Store proxy task handle and socket path
                            {
                                let mut running = self.running_lsps.write().await;
                                if let Some(process) = running.get_mut(&lsp_key) {
                                    process.proxy_task = Some(proxy_task);
                                    process.proxy_socket_path = Some(socket_path);
                                }
                            }

                            info!(
                                "LSP stdio proxy started for '{}', socket at: {:?}",
                                lsp_type.display_name(),
                                socket_path_for_log
                            );
                        }
                        Err(e) => {
                            // Proxy creation failed, but LSP is still running
                            warn!(
                                "Failed to create stdio proxy for LSP '{}': {}, continuing without proxy",
                                lsp_type.display_name(),
                                e
                            );
                            // Update process to disable proxy
                            let mut running = self.running_lsps.write().await;
                            if let Some(process) = running.get_mut(&lsp_key) {
                                process.use_proxy = false;
                            }
                        }
                    }
                }

                // Persist to database
                if let Err(e) = self.persist_lsp_state(session_id, lsp_type).await {
                    warn!("Failed to persist LSP state to database: {}", e);
                    // Continue anyway - graceful degradation
                }

                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to spawn LSP '{}': {}", lsp_type.display_name(), e);
                warn!("{}", error_msg);

                // Update status to error
                {
                    let mut running = self.running_lsps.write().await;
                    if let Some(process) = running.get_mut(&lsp_key) {
                        process.status = LspStatus::Error;
                        process.last_error = Some(e.to_string());
                    }
                }

                // Best-effort persist the error state.
                if let Err(e) = self.persist_lsp_state(session_id, lsp_type).await {
                    warn!("Failed to persist LSP error state to database: {}", e);
                }

                Err(anyhow!(error_msg))
            }
        }
    }

    /// Stop an LSP server for a session
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to stop
    pub async fn stop_lsp(&self, session_id: &str, lsp_type: LspType) -> Result<()> {
        let lsp_key = format!("{}:{}", session_id, lsp_type.binary_name());

        info!(
            "Stopping LSP '{}' for session '{}'",
            lsp_type.display_name(),
            session_id
        );

        // Extract the process from the map so we don't hold the lock across async operations.
        let process_opt = {
            let mut running = self.running_lsps.write().await;
            running.remove(&lsp_key)
        };

        if let Some(mut process) = process_opt {
            let pid = process.pid;

            // Actually kill the process (without holding the lock)
            if let Err(e) = process.kill().await {
                warn!("Failed to stop LSP process (PID: {:?}): {}", pid, e);
                // Re-insert on failure so we don't leak the process and can retry later.
                let mut running = self.running_lsps.write().await;
                running.insert(lsp_key.clone(), process);
                return Err(e);
            }

            // Persist final state (without holding the lock)
            let state = process.to_db_state();
            if let Err(e) = self.storage.upsert_lsp_state(&state).await {
                warn!("Failed to persist LSP stop state: {}", e);
            }
            debug!("LSP '{}' stopped", lsp_type.display_name());
        } else {
            debug!(
                "LSP '{}' not found for session '{}'",
                lsp_type.display_name(),
                session_id
            );
        }

        Ok(())
    }

    /// Get the status of an LSP server
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to query
    ///
    /// ## Returns
    ///
    /// Returns the LSP status, or `None` if not tracking this LSP
    pub async fn lsp_status(&self, session_id: &str, lsp_type: LspType) -> Option<LspStatus> {
        let lsp_key = format!("{}:{}", session_id, lsp_type.binary_name());
        let running = self.running_lsps.read().await;
        running.get(&lsp_key).map(|p| p.status)
    }

    /// Get all running LSPs for a session
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    ///
    /// ## Returns
    ///
    /// Returns a vector of (LspType, LspStatus) tuples
    pub async fn session_lsps(&self, session_id: &str) -> Vec<(LspType, LspStatus)> {
        let running = self.running_lsps.read().await;
        let mut result = Vec::new();

        for (key, process) in running.iter() {
            if key.starts_with(&format!("{}:", session_id)) {
                result.push((process.lsp_type, process.status));
            }
        }

        result
    }

    /// Get the stdio proxy socket path for an LSP
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to query
    ///
    /// ## Returns
    ///
    /// Returns the socket path if proxy is enabled, or `None` if not tracking this LSP or proxy is not enabled
    pub async fn get_proxy_socket_path(
        &self,
        session_id: &str,
        lsp_type: LspType,
    ) -> Option<PathBuf> {
        let lsp_key = format!("{}:{}", session_id, lsp_type.binary_name());
        let running = self.running_lsps.read().await;
        running
            .get(&lsp_key)
            .and_then(|p| p.proxy_socket_path.clone())
    }

    /// Persist LSP state to database
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to persist
    async fn persist_lsp_state(&self, session_id: &str, lsp_type: LspType) -> Result<()> {
        let lsp_key = format!("{}:{}", session_id, lsp_type.binary_name());

        // Extract the state we need, then release the lock before async operation
        let state_opt = {
            let running = self.running_lsps.read().await;
            running.get(&lsp_key).map(|process| process.to_db_state())
        };

        // Now that we've released the lock, do the async DB operation
        if let Some(state) = state_opt {
            // Persist to database with graceful degradation
            match self.storage.upsert_lsp_state(&state).await {
                Ok(id) => {
                    debug!(
                        "Persisted LSP state: {} (status: {}, id: {})",
                        state.lsp_name, state.status, id
                    );
                }
                Err(e) => {
                    warn!("Failed to persist LSP state to database: {}", e);
                    // Continue anyway - graceful degradation
                }
            }
        }

        Ok(())
    }

    /// Check the health of an LSP process
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to check
    ///
    /// ## Returns
    ///
    /// Returns `Ok(true)` if the LSP is running, `Ok(false)` if it has crashed or is not running,
    /// or an error if the check fails
    pub async fn check_lsp_health(&self, session_id: &str, lsp_type: LspType) -> Result<bool> {
        let lsp_key = format!("{}:{}", session_id, lsp_type.binary_name());

        // Remove the entry to avoid holding a lock across `.await`.
        let process_opt = {
            let mut running = self.running_lsps.write().await;
            running.remove(&lsp_key)
        };

        if let Some(mut process) = process_opt {
            let is_alive = process.is_alive().await;
            let state_to_persist = matches!(is_alive, Some(false)).then(|| process.to_db_state());

            {
                let mut running = self.running_lsps.write().await;
                running.insert(lsp_key, process);
            }

            if let Some(state) = state_to_persist {
                if let Err(e) = self.storage.upsert_lsp_state(&state).await {
                    warn!("Failed to persist crashed LSP state: {}", e);
                }
            }

            match is_alive {
                Some(is_alive) => {
                    debug!(
                        "LSP '{}' for session '{}' health check: {}",
                        lsp_type.display_name(),
                        session_id,
                        if is_alive { "healthy" } else { "not alive" }
                    );
                    Ok(is_alive)
                }
                None => {
                    debug!(
                        "No child process found for LSP '{}' in session '{}'",
                        lsp_type.display_name(),
                        session_id
                    );
                    Ok(false)
                }
            }
        } else {
            debug!(
                "LSP '{}' not found for session '{}'",
                lsp_type.display_name(),
                session_id
            );
            Ok(false)
        }
    }

    /// Detect and handle crashed LSP processes
    ///
    /// Scans all running LSPs and updates any that have crashed to Error status
    ///
    /// ## Returns
    ///
    /// Returns a vector of session_id:lsp_name strings for processes that were found to have crashed
    pub async fn detect_and_handle_crashes(&self) -> Result<Vec<String>> {
        let mut crashed_processes = Vec::new();

        // Snapshot keys to iterate without holding the lock.
        let lsp_keys: Vec<String> = {
            let running = self.running_lsps.read().await;
            running.keys().cloned().collect()
        };

        let mut states_to_persist: Vec<LspServerState> = Vec::new();

        for lsp_key in lsp_keys {
            let process_opt = {
                let mut running = self.running_lsps.write().await;
                running.remove(&lsp_key)
            };

            let Some(mut process) = process_opt else {
                continue;
            };

            match process.is_alive().await {
                Some(false) => {
                    crashed_processes.push(lsp_key.clone());
                    warn!("Detected crashed LSP process: {}", lsp_key);
                    states_to_persist.push(process.to_db_state());
                }
                Some(true) => {
                    debug!("LSP process is healthy: {}", lsp_key);
                }
                None => {
                    debug!("No child process to check for: {}", lsp_key);
                }
            }

            {
                let mut running = self.running_lsps.write().await;
                running.insert(lsp_key, process);
            }
        }

        // Persist crash status changes without holding any locks.
        for state in states_to_persist {
            if let Err(e) = self.storage.upsert_lsp_state(&state).await {
                warn!(
                    "Failed to persist crashed LSP state for session '{}' ({}): {}",
                    state.session_id, state.lsp_name, e
                );
            }
        }

        Ok(crashed_processes)
    }

    /// Shutdown all LSP processes with enhanced timeout handling
    ///
    /// Stops all running LSP processes for cleanup with improved timeout handling.
    /// This should be called before the LspManager is dropped.
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down LSP manager, stopping all processes");

        // Stop any background monitoring loop(s) started with `start_monitoring`.
        let _ = self.monitor_stop_tx.send(true);

        // Collect processes to kill, then release the lock before async operations
        let processes_to_kill: Vec<_> = {
            let mut running = self.running_lsps.write().await;
            running.drain().collect()
        };

        for (_lsp_key, mut process) in processes_to_kill {
            if let Err(e) = self.force_kill_process(&mut process).await {
                warn!("Failed to kill LSP process during shutdown: {}", e);
                // Continue anyway - clean up other processes
            }
        }

        info!("LSP manager shutdown complete");
        Ok(())
    }

    /// Force kill a process with timeout for force kill
    ///
    /// Attempts SIGTERM first, then SIGKILL if needed.
    ///
    /// Maestro does not currently send LSP `shutdown`/`exit` messages, so "graceful" here means
    /// "less forceful OS signal", not protocol-level graceful shutdown.
    async fn force_kill_process(&self, process: &mut LspProcess) -> Result<()> {
        process.kill_with_timeout(Duration::from_secs(10)).await
    }

    /// Start background monitoring of LSP processes
    ///
    /// This spawns a background task that periodically checks the health of all LSP processes
    /// and updates their status if they have crashed.
    ///
    /// ## Arguments
    ///
    /// - `interval`: How often to perform health checks
    ///
    /// ## Returns
    ///
    /// Returns a JoinHandle for the monitoring task
    pub fn start_monitoring(&self, interval_duration: Duration) -> JoinHandle<()> {
        let manager = self.clone(); // Clone the manager to move into the async block
        let mut stop_rx = manager.monitor_stop_tx.subscribe();

        tokio::spawn(async move {
            let mut interval_timer = interval(interval_duration);

            if *stop_rx.borrow() {
                return;
            }

            loop {
                tokio::select! {
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            debug!("Stopping LSP health monitor task");
                            break;
                        }
                    }
                    _ = interval_timer.tick() => {
                        match manager.detect_and_handle_crashes().await {
                            Ok(crashed_processes) => {
                                if !crashed_processes.is_empty() {
                                    info!("Health monitor detected {} crashed LSP processes", crashed_processes.len());
                                    for process in crashed_processes {
                                        warn!("Crashed LSP process: {}", process);
                                    }
                                } else {
                                    debug!("Health monitor found no crashed LSP processes");
                                }
                            }
                            Err(e) => {
                                warn!("Error during LSP health monitoring: {}", e);
                            }
                        }
                    }
                }
            }
        })
    }

    /// Detect languages from project directory by scanning for source files
    ///
    /// ## Arguments
    ///
    /// - `project_path`: Path to the project directory to scan
    ///
    /// ## Returns
    ///
    /// Returns a HashSet of LspType values representing the languages detected in the project
    ///
    /// ## Graceful Degradation
    ///
    /// If directory traversal fails or any error occurs during scanning, this method returns an empty set
    pub async fn detect_languages_from_project(
        &self,
        project_path: &std::path::Path,
    ) -> Result<std::collections::HashSet<LspType>> {
        use std::collections::HashSet;
        use walkdir::WalkDir;

        let mut detected_languages = HashSet::new();

        debug!(
            "Scanning project directory for language detection: {:?}",
            project_path
        );

        // Walk through the project directory recursively, skipping hidden directories/files
        // entirely (e.g. `.git/`).
        let walker = WalkDir::new(project_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                if e.depth() == 0 {
                    return true;
                }
                e.file_name()
                    .to_str()
                    .map(|name| !name.starts_with('.'))
                    .unwrap_or(true)
            });

        for entry in walker {
            match entry {
                Ok(entry) => {
                    if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
                        // Check each LSP type against the file extension
                        for lsp_type in [LspType::Rust, LspType::Python, LspType::TypeScript] {
                            if lsp_type.file_extensions().iter().any(|&e| e == ext) {
                                debug!(
                                    "Detected language {:?} from file: {:?}",
                                    lsp_type,
                                    entry.path()
                                );
                                detected_languages.insert(lsp_type);
                                break; // Found a match, no need to check other LSP types for this file
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Error reading directory entry during language detection: {}",
                        e
                    );
                    // Continue with other entries - graceful degradation
                    continue;
                }
            }
        }

        debug!("Detected languages in project: {:?}", detected_languages);
        Ok(detected_languages)
    }

    /// Recommend LSPs for a session based on project language detection
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `project_path`: Path to the project directory to scan
    ///
    /// ## Returns
    ///
    /// Returns a sorted vector of LspType values to recommend starting for this session
    ///
    /// ## Sorting Order
    ///
    /// Languages are sorted in the order: Rust, Python, TypeScript
    pub async fn recommend_lsps_for_session(
        &self,
        session_id: &str,
        project_path: &std::path::Path,
    ) -> Result<Vec<LspType>> {
        debug!(
            "Recommending LSPs for session '{}' based on project: {:?}",
            session_id, project_path
        );

        let detected_languages = match self.detect_languages_from_project(project_path).await {
            Ok(langs) => langs,
            Err(e) => {
                warn!(
                    "Failed to detect languages for session '{}': {}",
                    session_id, e
                );
                // Return empty vector on error - graceful degradation
                std::collections::HashSet::new()
            }
        };

        // Sort LSP types in the specified order: Rust, Python, TypeScript
        let mut recommended_lsps: Vec<LspType> = detected_languages.into_iter().collect();
        recommended_lsps.sort_by(|a, b| {
            // Define the sort order: Rust, Python, TypeScript
            match (a, b) {
                (LspType::Rust, LspType::Rust) => Ordering::Equal,
                (LspType::Rust, _) => Ordering::Less,
                (LspType::Python, LspType::Rust) => Ordering::Greater,
                (LspType::Python, LspType::Python) => Ordering::Equal,
                (LspType::Python, _) => Ordering::Less,
                (LspType::TypeScript, LspType::TypeScript) => Ordering::Equal,
                (_, _) => Ordering::Greater,
            }
        });

        debug!(
            "Recommended LSPs for session '{}': {:?}",
            session_id, recommended_lsps
        );
        Ok(recommended_lsps)
    }

    /// Auto-start LSPs for a session based on project language detection
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `project_path`: Path to the project directory to scan
    ///
    /// ## Returns
    ///
    /// Returns a vector of LspType values representing the LSPs that were started
    ///
    /// ## Graceful Degradation
    ///
    /// If any LSP fails to start, this method continues with other LSPs and returns the ones that succeeded
    pub async fn auto_start_lsps_for_session(
        &self,
        session_id: &str,
        project_path: &std::path::Path,
    ) -> Result<Vec<LspType>> {
        debug!(
            "Auto-starting LSPs for session '{}' based on project: {:?}",
            session_id, project_path
        );

        // Get recommended LSPs for the session
        let recommended_lsps = self
            .recommend_lsps_for_session(session_id, project_path)
            .await
            .unwrap_or_else(|e| {
                warn!(
                    "Failed to recommend LSPs for session '{}': {}",
                    session_id, e
                );
                vec![]
            });

        let mut started_lsps = Vec::new();

        // Start each recommended LSP
        for lsp_type in recommended_lsps {
            match self.start_lsp(session_id, lsp_type, None).await {
                Ok(()) => {
                    info!(
                        "Successfully auto-started LSP '{}' for session '{}'",
                        lsp_type.display_name(),
                        session_id
                    );
                    started_lsps.push(lsp_type);
                }
                Err(e) => {
                    warn!(
                        "Failed to auto-start LSP '{}' for session '{}': {}",
                        lsp_type.display_name(),
                        session_id,
                        e
                    );
                    // Continue with other LSPs - graceful degradation
                }
            }
        }

        debug!(
            "Auto-started {} LSPs for session '{}': {:?}",
            started_lsps.len(),
            session_id,
            started_lsps
        );
        Ok(started_lsps)
    }

    /// Restore LSPs on startup by restarting those that were previously running with auto_start=true
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())` on success, or error with details
    ///
    /// ## Graceful Degradation
    ///
    /// If any LSP fails to restart, this method continues with other LSPs and logs the failure
    pub async fn restore_lsps_on_startup(&self) -> Result<()> {
        info!("Restoring LSPs on startup");

        // Get all LSP states with auto_start=true from the database
        let auto_start_states = match self.storage.get_lsp_states_by_auto_start(true).await {
            Ok(states) => states,
            Err(e) => {
                warn!("Failed to get auto-start LSP states from database: {}", e);
                // Continue with empty list - graceful degradation
                vec![]
            }
        };

        debug!(
            "Found {} auto-start LSP states to restore",
            auto_start_states.len()
        );

        for state in auto_start_states {
            // Parse the LSP type from the language name
            let lsp_type = match state.language.as_str() {
                "rust" => LspType::Rust,
                "python" => LspType::Python,
                "typescript" => LspType::TypeScript,
                _ => {
                    warn!(
                        "Unknown language '{}' for LSP state, skipping restoration",
                        state.language
                    );
                    continue;
                }
            };

            // Only restart if the LSP was previously running
            if state.status == LspStatus::Running || state.status == LspStatus::Starting {
                info!(
                    "Restoring LSP '{}' for session '{}'",
                    lsp_type.display_name(),
                    state.session_id
                );

                match self.start_lsp(&state.session_id, lsp_type, None).await {
                    Ok(()) => {
                        info!(
                            "Successfully restored LSP '{}' for session '{}'",
                            lsp_type.display_name(),
                            state.session_id
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to restore LSP '{}' for session '{}': {}",
                            lsp_type.display_name(),
                            state.session_id,
                            e
                        );
                        // Continue with other LSPs - graceful degradation
                    }
                }
            } else {
                debug!(
                    "Skipping restoration of LSP '{}' for session '{}' (status: {})",
                    lsp_type.display_name(),
                    state.session_id,
                    state.status
                );
            }
        }

        info!("LSP restoration on startup completed");
        Ok(())
    }

    /// Restart an LSP server for a session
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to restart
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())` if LSP restarted successfully, or error with details
    ///
    /// ## Graceful Degradation
    ///
    /// If the LSP fails to stop or start, this method logs the error but continues
    pub async fn restart_lsp(&self, session_id: &str, lsp_type: LspType) -> Result<()> {
        info!(
            "Restarting LSP '{}' for session '{}'",
            lsp_type.display_name(),
            session_id
        );

        // First, try to stop the LSP
        match self.stop_lsp(session_id, lsp_type).await {
            Ok(()) => {
                debug!(
                    "LSP '{}' stopped successfully for session '{}'",
                    lsp_type.display_name(),
                    session_id
                );
            }
            Err(e) => {
                warn!(
                    "Failed to stop LSP '{}' for session '{}': {}",
                    lsp_type.display_name(),
                    session_id,
                    e
                );
                // Continue anyway - try to start the LSP
            }
        }

        // Then, start the LSP again with the same configuration
        // First, get the existing configuration from the database if it exists
        let config = match self
            .storage
            .get_lsp_state(session_id, lsp_type.binary_name())
            .await
        {
            Ok(Some(state)) => {
                // If we have a saved state, we can preserve the auto_start setting
                Some(LspConfig {
                    auto_start: state.auto_start,
                    ..Default::default()
                })
            }
            Ok(None) => {
                // No existing state, use default config
                Some(LspConfig::default())
            }
            Err(e) => {
                warn!(
                    "Failed to get existing LSP state for '{}', using default config: {}",
                    lsp_type.binary_name(),
                    e
                );
                Some(LspConfig::default())
            }
        };

        // Start the LSP with the preserved configuration
        match self.start_lsp(session_id, lsp_type, config).await {
            Ok(()) => {
                info!(
                    "LSP '{}' restarted successfully for session '{}'",
                    lsp_type.display_name(),
                    session_id
                );
                Ok(())
            }
            Err(e) => {
                warn!(
                    "Failed to start LSP '{}' for session '{}' after restart: {}",
                    lsp_type.display_name(),
                    session_id,
                    e
                );
                // Don't return error - graceful degradation
                Ok(())
            }
        }
    }

    /// Set the auto-start flag for an LSP in a session
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to update
    /// - `enabled`: Whether auto-start should be enabled
    ///
    /// ## Returns
    ///
    /// Returns `Ok(true)` if the flag was updated, `Ok(false)` if no change was needed
    ///
    /// ## Graceful Degradation
    ///
    /// If database persistence fails, this method updates in-memory state and continues
    pub async fn set_auto_start(
        &self,
        session_id: &str,
        lsp_type: LspType,
        enabled: bool,
    ) -> Result<bool> {
        let lsp_key = format!("{}:{}", session_id, lsp_type.binary_name());

        info!(
            "Setting auto-start for LSP '{}' in session '{}' to {}",
            lsp_type.display_name(),
            session_id,
            enabled
        );

        // Update in-memory state and release lock before async DB operations
        let memory_changed = {
            let mut running = self.running_lsps.write().await;
            if let Some(process) = running.get_mut(&lsp_key) {
                let old_value = process.auto_start;
                process.auto_start = enabled;
                old_value != enabled
            } else {
                // If not running, we still want to update the database state
                false
            }
        };
        // Lock released here

        let mut db_updated = false;

        // Update in database WITHOUT holding lock
        match self
            .storage
            .get_lsp_state(session_id, lsp_type.binary_name())
            .await
        {
            Ok(Some(mut state)) => {
                // Update the existing state
                if state.auto_start != enabled {
                    state.auto_start = enabled;
                    match self.storage.upsert_lsp_state(&state).await {
                        Ok(_) => {
                            debug!(
                                "Updated auto-start flag for LSP '{}' in session '{}' to {}",
                                lsp_type.binary_name(),
                                session_id,
                                enabled
                            );
                            db_updated = true;
                        }
                        Err(e) => {
                            warn!("Failed to persist auto-start flag to database: {}", e);
                            // Continue anyway - graceful degradation
                        }
                    }
                }
            }
            Ok(None) => {
                // Create a new state record with the auto_start flag
                let new_state = LspServerState {
                    id: 0, // Will be set by database
                    session_id: session_id.to_string(),
                    language: lsp_type.language().to_string(),
                    lsp_name: lsp_type.binary_name().to_string(),
                    status: LspStatus::Stopped, // Default status
                    pid: None,
                    port: None,
                    auto_start: enabled,
                    last_started: None,
                    last_error: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    updated_at: Some(chrono::Utc::now().to_rfc3339()),
                };

                match self.storage.upsert_lsp_state(&new_state).await {
                    Ok(_) => {
                        debug!(
                            "Created new LSP state with auto-start flag for '{}' in session '{}'",
                            lsp_type.binary_name(),
                            session_id
                        );
                        db_updated = true;
                    }
                    Err(e) => {
                        warn!("Failed to persist new LSP state to database: {}", e);
                        // Continue anyway - graceful degradation
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to get existing LSP state for '{}': {}",
                    lsp_type.binary_name(),
                    e
                );
                // Continue anyway - graceful degradation
            }
        }

        Ok(memory_changed || db_updated)
    }

    /// Disable auto-start for an LSP in a session
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to update
    ///
    /// ## Returns
    ///
    /// Returns `Ok(true)` if the flag was updated, `Ok(false)` if no change was needed
    ///
    /// ## Graceful Degradation
    ///
    /// If database persistence fails, this method updates in-memory state and continues
    pub async fn disable_auto_start(&self, session_id: &str, lsp_type: LspType) -> Result<bool> {
        self.set_auto_start(session_id, lsp_type, false).await
    }

    // ========================================================================
    // Proxy Mode Management
    // ========================================================================

    /// Enable stdio proxy mode for an LSP in a session
    ///
    /// When proxy mode is enabled, the LSP will be accessible via a Unix socket
    /// that allows multiple concurrent clients. This requires restarting the LSP.
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to enable proxy for
    ///
    /// ## Returns
    ///
    /// Returns `Ok(true)` if the proxy mode was enabled, `Ok(false)` if already enabled
    ///
    /// ## Behavior
    ///
    /// If the LSP is currently running, it will be restarted with proxy mode enabled.
    /// The restart is graceful - the LSP is stopped first, then started with the new configuration.
    pub async fn enable_proxy_mode(&self, session_id: &str, lsp_type: LspType) -> Result<bool> {
        let lsp_key = format!("{}:{}", session_id, lsp_type.binary_name());

        info!(
            "Enabling proxy mode for LSP '{}' in session '{}'",
            lsp_type.display_name(),
            session_id
        );

        // Check if proxy is already enabled
        let already_enabled = {
            let running = self.running_lsps.read().await;
            running.get(&lsp_key).map(|p| p.use_proxy).unwrap_or(false)
        };

        if already_enabled {
            debug!(
                "Proxy mode already enabled for LSP '{}' in session '{}'",
                lsp_type.display_name(),
                session_id
            );
            return Ok(false);
        }

        // Restart the LSP with proxy mode enabled
        // First, get the existing configuration from the database if it exists
        let _config = match self
            .storage
            .get_lsp_state(session_id, lsp_type.binary_name())
            .await
        {
            Ok(Some(state)) => {
                // If we have a saved state, we can preserve the existing settings
                Some(LspConfig {
                    auto_start: state.auto_start,
                    use_proxy: true,
                    ..Default::default()
                })
            }
            Ok(None) => {
                // No existing state, use default config with proxy enabled
                Some(LspConfig {
                    auto_start: true,
                    use_proxy: true,
                    ..Default::default()
                })
            }
            Err(e) => {
                warn!(
                    "Failed to get existing LSP state for '{}', using default config: {}",
                    lsp_type.binary_name(),
                    e
                );
                Some(LspConfig {
                    auto_start: true,
                    use_proxy: true,
                    ..Default::default()
                })
            }
        };

        // Restart the LSP with proxy enabled
        match self.restart_lsp(session_id, lsp_type).await {
            Ok(()) => {
                // Update the in-memory proxy flag
                let mut running = self.running_lsps.write().await;
                if let Some(process) = running.get_mut(&lsp_key) {
                    process.use_proxy = true;
                }
                drop(running);

                info!(
                    "Proxy mode enabled for LSP '{}' in session '{}'",
                    lsp_type.display_name(),
                    session_id
                );
                Ok(true)
            }
            Err(e) => {
                warn!(
                    "Failed to restart LSP '{}' for proxy mode: {}",
                    lsp_type.display_name(),
                    e
                );
                // Don't return error - graceful degradation
                Ok(false)
            }
        }
    }

    /// Disable stdio proxy mode for an LSP in a session
    ///
    /// When proxy mode is disabled, the LSP will use direct stdio communication
    /// (single client only). This requires restarting the LSP.
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to disable proxy for
    ///
    /// ## Returns
    ///
    /// Returns `Ok(true)` if the proxy mode was disabled, `Ok(false)` if already disabled
    ///
    /// ## Behavior
    ///
    /// If the LSP is currently running, it will be restarted with proxy mode disabled.
    /// The proxy socket will be cleaned up during the restart.
    pub async fn disable_proxy_mode(&self, session_id: &str, lsp_type: LspType) -> Result<bool> {
        let lsp_key = format!("{}:{}", session_id, lsp_type.binary_name());

        info!(
            "Disabling proxy mode for LSP '{}' in session '{}'",
            lsp_type.display_name(),
            session_id
        );

        // Check if proxy is currently enabled
        let is_enabled = {
            let running = self.running_lsps.read().await;
            running.get(&lsp_key).map(|p| p.use_proxy).unwrap_or(false)
        };

        if !is_enabled {
            debug!(
                "Proxy mode already disabled for LSP '{}' in session '{}'",
                lsp_type.display_name(),
                session_id
            );
            return Ok(false);
        }

        // Restart the LSP with proxy mode disabled
        match self.restart_lsp(session_id, lsp_type).await {
            Ok(()) => {
                // Update the in-memory proxy flag
                let mut running = self.running_lsps.write().await;
                if let Some(process) = running.get_mut(&lsp_key) {
                    process.use_proxy = false;
                }
                drop(running);

                info!(
                    "Proxy mode disabled for LSP '{}' in session '{}'",
                    lsp_type.display_name(),
                    session_id
                );
                Ok(true)
            }
            Err(e) => {
                warn!(
                    "Failed to restart LSP '{}' to disable proxy mode: {}",
                    lsp_type.display_name(),
                    e
                );
                // Don't return error - graceful degradation
                Ok(false)
            }
        }
    }

    /// Check if proxy mode is enabled for an LSP
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to query
    ///
    /// ## Returns
    ///
    /// Returns `true` if proxy mode is enabled, `false` otherwise
    pub async fn is_proxy_enabled(&self, session_id: &str, lsp_type: LspType) -> bool {
        let lsp_key = format!("{}:{}", session_id, lsp_type.binary_name());
        let running = self.running_lsps.read().await;
        running.get(&lsp_key).map(|p| p.use_proxy).unwrap_or(false)
    }

    /// Stop all LSPs for a specific session
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())` on success, or error with details
    ///
    /// ## Graceful Degradation
    ///
    /// If any LSP fails to stop, this method continues with other LSPs and logs the failure
    pub async fn stop_all_session_lsps(&self, session_id: &str) -> Result<()> {
        info!("Stopping all LSPs for session '{}'", session_id);

        // Get all running LSPs for this session
        let session_lsps = self.session_lsps(session_id).await;

        // Stop each LSP
        for (lsp_type, _) in session_lsps {
            match self.stop_lsp(session_id, lsp_type).await {
                Ok(()) => {
                    debug!(
                        "Successfully stopped LSP '{}' for session '{}'",
                        lsp_type.display_name(),
                        session_id
                    );
                }
                Err(e) => {
                    warn!(
                        "Failed to stop LSP '{}' for session '{}': {}",
                        lsp_type.display_name(),
                        session_id,
                        e
                    );
                    // Continue with other LSPs - graceful degradation
                }
            }
        }

        info!("All LSPs stopped for session '{}'", session_id);
        Ok(())
    }

    // ========================================================================
    // MCP Bridge Integration
    // ========================================================================

    /// Start an MCP bridge for an LSP
    ///
    /// This starts the maestro-lsp-mcp-bridge binary that translates LSP
    /// capabilities to MCP protocol.
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to bridge
    /// - `project_path`: Path to the project root
    ///
    /// ## Returns
    ///
    /// Returns the bridge process ID on success, or an error
    ///
    /// ## Graceful Degradation
    ///
    /// If the bridge fails to start, this method logs the error and continues.
    /// The LSP itself will still function normally, just without MCP exposure.
    pub async fn start_mcp_bridge(
        &self,
        session_id: &str,
        lsp_type: LspType,
        project_path: &std::path::Path,
    ) -> Result<u32> {
        let bridge_key = format!("{}:{}:mcp", session_id, lsp_type.binary_name());

        info!(
            "Starting MCP bridge for LSP '{}' in session '{}'",
            lsp_type.display_name(),
            session_id
        );

        // Get the path to the maestro-lsp-mcp-bridge binary
        let bridge_binary = "maestro-lsp-mcp-bridge";

        // Build the command to start the MCP bridge
        let mut cmd = TokioCommand::new(bridge_binary);
        cmd.arg("--lsp-type")
            .arg(match lsp_type {
                LspType::Rust => "rust",
                LspType::Python => "python",
                LspType::TypeScript => "typescript",
            })
            .arg("--project-path")
            .arg(project_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // Inherit stderr to avoid deadlock
            .kill_on_drop(true);

        #[cfg(unix)]
        unsafe {
            cmd.pre_exec(|| {
                // Create a new process group so we can kill the entire bridge tree on shutdown.
                if libc::setpgid(0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                info!(
                    "MCP bridge for LSP '{}' started successfully (PID: {})",
                    lsp_type.display_name(),
                    pid.unwrap_or(0)
                );

                // Store the child handle so we can properly manage the bridge lifecycle
                let mut bridges = self.running_bridges.write().await;
                bridges.insert(bridge_key, child);

                Ok(pid.unwrap_or(0))
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to start MCP bridge for LSP '{}': {}",
                    lsp_type.display_name(),
                    e
                );
                warn!("{}", error_msg);

                // Continue anyway - LSP will still function without MCP bridge
                Err(anyhow!(error_msg))
            }
        }
    }

    /// Stop an MCP bridge for an LSP
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP whose bridge to stop
    /// - `bridge_pid`: Process ID of the bridge process
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())` on success
    ///
    /// ## Graceful Degradation
    ///
    /// If stopping the bridge fails, this method logs the error and continues
    pub async fn stop_mcp_bridge(
        &self,
        session_id: &str,
        lsp_type: LspType,
        bridge_pid: u32,
    ) -> Result<()> {
        let bridge_key = format!("{}:{}:mcp", session_id, lsp_type.binary_name());

        info!(
            "Stopping MCP bridge for LSP '{}' in session '{}' (PID: {})",
            lsp_type.display_name(),
            session_id,
            bridge_pid
        );

        // Remove and get the child handle from the map
        let child = {
            let mut bridges = self.running_bridges.write().await;
            bridges.remove(&bridge_key)
        };

        if let Some(mut child_handle) = child {
            // Kill the bridge process using the stored child handle
            #[cfg(unix)]
            {
                if let Some(pgid) = get_process_group_id(child_handle.id()) {
                    // Try SIGTERM first
                    if let Err(e) = unix_kill_process_group(pgid, libc::SIGTERM) {
                        warn!(
                            "Failed to SIGTERM bridge process group (PGID: {}): {}",
                            pgid, e
                        );
                    }

                    // Wait up to 5 seconds for graceful shutdown
                    match timeout(Duration::from_secs(5), child_handle.wait()).await {
                        Ok(Ok(exit_status)) => {
                            info!("MCP bridge exited after SIGTERM: {:?}", exit_status);
                        }
                        Ok(Err(e)) => {
                            warn!("Failed to wait for bridge process: {}", e);
                        }
                        Err(_) => {
                            // Timeout - send SIGKILL
                            debug!("Bridge did not exit within timeout; sending SIGKILL");
                            if let Err(e) = unix_kill_process_group(pgid, libc::SIGKILL) {
                                warn!("Failed to SIGKILL bridge process group: {}", e);
                            }
                            let _ = timeout(Duration::from_secs(2), child_handle.wait()).await;
                        }
                    }
                }
            }

            #[cfg(not(unix))]
            {
                let _ = child_handle.kill().await;
                let _ = timeout(Duration::from_secs(5), child_handle.wait()).await;
            }

            info!("MCP bridge stopped successfully");
        } else {
            warn!("No bridge process found for key: {}", bridge_key);
        }

        Ok(())
    }

    /// Start both an LSP and its MCP bridge
    ///
    /// This is a convenience method that starts both the LSP and its MCP bridge
    /// in a single call.
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to start
    /// - `project_path`: Path to the project root
    /// - `config`: Optional LSP configuration
    ///
    /// ## Returns
    ///
    /// Returns `(lsp_result, bridge_pid)` where lsp_result is the LSP start result
    /// and bridge_pid is the bridge process ID (or 0 if bridge failed to start)
    pub async fn start_lsp_with_mcp_bridge(
        &self,
        session_id: &str,
        lsp_type: LspType,
        project_path: &std::path::Path,
        config: Option<LspConfig>,
    ) -> (Result<()>, u32) {
        // First, start the LSP
        let lsp_result = self.start_lsp(session_id, lsp_type, config).await;

        // Then, start the MCP bridge (don't fail if LSP failed)
        let bridge_pid = if lsp_result.is_ok() {
            match self
                .start_mcp_bridge(session_id, lsp_type, project_path)
                .await
            {
                Ok(pid) => pid,
                Err(_) => 0, // Bridge failed to start, but LSP is running
            }
        } else {
            0
        };

        (lsp_result, bridge_pid)
    }

    /// Stop both an LSP and its MCP bridge
    ///
    /// This is a convenience method that stops both the LSP and its MCP bridge
    /// in a single call.
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to stop
    /// - `bridge_pid`: Process ID of the bridge process (0 if no bridge)
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())` on success
    pub async fn stop_lsp_with_mcp_bridge(
        &self,
        session_id: &str,
        lsp_type: LspType,
        bridge_pid: u32,
    ) -> Result<()> {
        // Stop the MCP bridge first
        if bridge_pid > 0 {
            let _ = self.stop_mcp_bridge(session_id, lsp_type, bridge_pid).await;
        }

        // Then stop the LSP
        self.stop_lsp(session_id, lsp_type).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_lsp_type_properties() {
        assert_eq!(LspType::Rust.binary_name(), "rust-analyzer");
        assert_eq!(LspType::Python.binary_name(), "ruff-lsp");
        assert_eq!(
            LspType::TypeScript.binary_name(),
            "typescript-language-server"
        );

        assert_eq!(LspType::Rust.language(), "rust");
        assert_eq!(LspType::Python.language(), "python");
        assert_eq!(LspType::TypeScript.language(), "typescript");

        assert_eq!(LspType::Rust.file_extensions(), &["rs"]);
        assert_eq!(LspType::Python.file_extensions(), &["py"]);
        assert_eq!(
            LspType::TypeScript.file_extensions(),
            &["ts", "tsx", "js", "jsx"]
        );
    }

    #[test]
    fn test_lsp_config_default() {
        let config = LspConfig::default();
        assert!(config.auto_start);
        assert!(config.binary_path.is_none());
        assert!(config.additional_args.is_empty());
        assert!(config.env_vars.is_empty());
    }

    #[test]
    fn test_lsp_process_creation() {
        let process = LspProcess::new(LspType::Rust, "test-session".to_string(), false);
        assert_eq!(process.lsp_type, LspType::Rust);
        assert_eq!(process.session_id, "test-session");
        assert_eq!(process.status, LspStatus::Stopped);
        assert!(process.pid.is_none());
        assert!(process.last_error.is_none());
        assert!(!process.use_proxy);
        assert!(process.proxy_task.is_none());
        assert!(process.proxy_socket_path.is_none());
    }

    #[tokio::test]
    async fn test_lsp_manager_creation() {
        let storage = TursoStorageBackend::in_memory(None)
            .await
            .expect("Failed to create storage");
        let manager = LspManager::new(storage);
        // Just verify it was created successfully
        assert_eq!(manager.running_lsps.read().await.len(), 0);
    }

    #[test]
    fn test_lsp_status_display() {
        assert_eq!(LspStatus::Running.as_str(), "running");
        assert_eq!(LspStatus::Stopped.as_str(), "stopped");
        assert_eq!(LspStatus::Error.as_str(), "error");
        assert_eq!(LspStatus::Starting.as_str(), "starting");
    }

    #[test]
    fn test_lsp_status_from_str() {
        assert_eq!(LspStatus::from_str("running"), Some(LspStatus::Running));
        assert_eq!(LspStatus::from_str("stopped"), Some(LspStatus::Stopped));
        assert_eq!(LspStatus::from_str("error"), Some(LspStatus::Error));
        assert_eq!(LspStatus::from_str("starting"), Some(LspStatus::Starting));
        assert_eq!(LspStatus::from_str("invalid"), None);
    }

    #[tokio::test]
    async fn test_detect_languages_from_project() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory for testing
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_path = temp_dir.path();

        // Create test files with different extensions
        fs::write(project_path.join("main.rs"), "fn main() {}").unwrap();
        fs::write(project_path.join("script.py"), "print('hello')").unwrap();
        fs::write(project_path.join("index.ts"), "console.log('hello');").unwrap();
        fs::write(project_path.join("utils.js"), "console.log('utils');").unwrap();
        fs::write(project_path.join("README.md"), "# Test").unwrap(); // Should be ignored

        let storage = TursoStorageBackend::in_memory(None)
            .await
            .expect("Failed to create storage");
        let manager = LspManager::new(storage);

        let detected = manager
            .detect_languages_from_project(project_path)
            .await
            .unwrap();

        // Check that the correct languages were detected
        assert!(detected.contains(&LspType::Rust));
        assert!(detected.contains(&LspType::Python));
        assert!(detected.contains(&LspType::TypeScript));
        assert_eq!(detected.len(), 3); // Only 3 languages should be detected
    }

    #[tokio::test]
    async fn test_recommend_lsps_for_session() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory for testing
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_path = temp_dir.path();

        // Create test files with different extensions
        fs::write(project_path.join("script.py"), "print('hello')").unwrap();
        fs::write(project_path.join("main.rs"), "fn main() {}").unwrap();
        fs::write(project_path.join("index.ts"), "console.log('hello');").unwrap();

        let storage = TursoStorageBackend::in_memory(None)
            .await
            .expect("Failed to create storage");
        let manager = LspManager::new(storage);

        let recommended = manager
            .recommend_lsps_for_session("test-session", project_path)
            .await
            .unwrap();

        // Check that all three languages were detected
        assert_eq!(recommended.len(), 3);

        // Check that the order is Rust, Python, TypeScript as specified
        // Since we sort by Rust, Python, TypeScript order, the first should be Rust
        assert_eq!(recommended[0], LspType::Rust);
        assert_eq!(recommended[1], LspType::Python);
        assert_eq!(recommended[2], LspType::TypeScript);
    }

    #[tokio::test]
    async fn test_detect_languages_empty_project() {
        // Test with an empty directory
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_path = temp_dir.path();

        let storage = TursoStorageBackend::in_memory(None)
            .await
            .expect("Failed to create storage");
        let manager = LspManager::new(storage);

        let detected = manager
            .detect_languages_from_project(project_path)
            .await
            .unwrap();

        // Should return empty set for empty directory
        assert_eq!(detected.len(), 0);
    }

    #[tokio::test]
    async fn test_detect_languages_skips_hidden_dirs() {
        use std::fs;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_path = temp_dir.path();

        fs::write(project_path.join("main.rs"), "fn main() {}").unwrap();
        fs::create_dir_all(project_path.join(".git")).unwrap();
        fs::write(
            project_path.join(".git").join("ignored.py"),
            "print('nope')",
        )
        .unwrap();
        fs::write(project_path.join(".hidden.ts"), "console.log('nope');").unwrap();

        let storage = TursoStorageBackend::in_memory(None)
            .await
            .expect("Failed to create storage");
        let manager = LspManager::new(storage);

        let detected = manager
            .detect_languages_from_project(project_path)
            .await
            .unwrap();

        assert!(detected.contains(&LspType::Rust));
        assert!(!detected.contains(&LspType::Python));
        assert!(!detected.contains(&LspType::TypeScript));
    }
}
