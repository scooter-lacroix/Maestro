//! OMP Worker Process Management
//!
//! Spawns and manages OMP worker subprocesses with IPC communication.

use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info};

use super::protocol::{OmpRequest, OmpResponse, OmpWorkerInit, OmpWorkerStatus};

/// Default timeout for worker responses
const DEFAULT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(60);

/// Default timeout for worker startup
const DEFAULT_STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

/// OMP worker configuration
#[derive(Debug, Clone)]
pub struct OmpWorkerConfig {
    /// Path to OMP installation (oh-my-pi directory)
    pub omp_path: PathBuf,
    /// Session ID for this worker
    pub session_id: String,
    /// Project path (working directory)
    pub project_path: PathBuf,
    /// Model to use
    pub model: String,
    /// Enabled tools
    pub tools: Vec<String>,
    /// Optional LSP pool socket path
    pub lsp_pool_socket: Option<String>,
    /// Optional MCP pool socket path
    pub mcp_pool_socket: Option<String>,
    /// Environment variables
    pub env: Vec<(String, String)>,
    /// Response timeout
    pub response_timeout: Duration,
}

impl Default for OmpWorkerConfig {
    fn default() -> Self {
        Self {
            omp_path: PathBuf::from("vendor/oh-my-pi"),
            session_id: "default".to_string(),
            project_path: PathBuf::from("."),
            model: "claude-3-5-sonnet".to_string(),
            tools: vec![
                "python".to_string(),
                "edit".to_string(),
                "grep".to_string(),
                "find".to_string(),
                "read".to_string(),
                "write".to_string(),
            ],
            lsp_pool_socket: None,
            mcp_pool_socket: None,
            env: Vec::new(),
            response_timeout: DEFAULT_RESPONSE_TIMEOUT,
        }
    }
}

/// Pending response tracker
struct PendingResponse {
    /// One-shot channel to send response back
    tx: oneshot::Sender<Result<OmpResponse>>,
    /// When this request was sent (for timeout tracking)
    sent_at: Instant,
}

impl PendingResponse {
    /// Create a new pending response
    fn new(tx: oneshot::Sender<Result<OmpResponse>>) -> Self {
        Self {
            tx,
            sent_at: Instant::now(),
        }
    }

    /// Check if this request has timed out
    fn is_timeout(&self, timeout: Duration) -> bool {
        self.sent_at.elapsed() > timeout
    }

    /// Get elapsed time since request was sent
    fn elapsed(&self) -> Duration {
        self.sent_at.elapsed()
    }
}

/// OMP worker process handle
pub struct OmpWorker {
    /// Worker configuration
    config: OmpWorkerConfig,
    /// Child process handle
    child: Option<Child>,
    /// Stdin for sending requests
    stdin: Option<ChildStdin>,
    /// Next request ID
    next_id: Arc<RwLock<u64>>,
    /// Pending responses (request ID -> sender)
    pending: Arc<RwLock<std::collections::HashMap<u64, PendingResponse>>>,
    /// Output stream receiver
    output_rx: Option<mpsc::Receiver<String>>,
    /// Worker status
    status: Arc<RwLock<OmpWorkerStatus>>,
    /// Shutdown signal
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl OmpWorker {
    /// Create a new OMP worker (not yet started)
    pub fn new(config: OmpWorkerConfig) -> Self {
        Self {
            config,
            child: None,
            stdin: None,
            next_id: Arc::new(RwLock::new(1)),
            pending: Arc::new(RwLock::new(std::collections::HashMap::new())),
            output_rx: None,
            status: Arc::new(RwLock::new(OmpWorkerStatus::uninitialized())),
            shutdown_tx: None,
        }
    }

    /// Start the OMP worker process
    pub async fn start(&mut self) -> Result<()> {
        if self.child.is_some() {
            return Err(anyhow!("Worker already started"));
        }

        info!(
            "Starting OMP worker for session: {}",
            self.config.session_id
        );

        // Find bun binary
        let bun = which::which("bun").context("bun not found in PATH")?;

        // Build init config
        let init = OmpWorkerInit {
            session_id: self.config.session_id.clone(),
            project_path: self.config.project_path.to_string_lossy().to_string(),
            model: self.config.model.clone(),
            tools: self.config.tools.clone(),
            lsp_pool_socket: self.config.lsp_pool_socket.clone(),
            mcp_pool_socket: self.config.mcp_pool_socket.clone(),
            env: self.config.env.iter().cloned().collect(),
        };

        // Spawn worker process
        let mut cmd = Command::new(bun);
        cmd.arg("run")
            .arg("--silent")
            .arg(
                &self
                    .config
                    .omp_path
                    .join("packages/coding-agent/src/worker.ts"),
            )
            .arg("--session-id")
            .arg(&self.config.session_id)
            .current_dir(&self.config.omp_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        // Add environment variables
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        // Add init config as env var (worker reads on startup)
        cmd.env("OMP_INIT_CONFIG", serde_json::to_string(&init)?);

        let mut child = cmd.spawn().context("Failed to spawn OMP worker")?;

        let stdin = child.stdin.take().context("Failed to open stdin")?;
        let stdout = child.stdout.take().context("Failed to open stdout")?;

        self.stdin = Some(stdin);
        self.child = Some(child);

        // Create output channel
        let (output_tx, output_rx) = mpsc::channel(100);
        self.output_rx = Some(output_rx);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        // Start response reader task
        let pending = self.pending.clone();
        let status = self.status.clone();
        tokio::spawn(async move {
            Self::response_reader(stdout, pending, output_tx, status, shutdown_rx).await;
        });

        // Wait for worker ready signal with timeout
        let ready = timeout(DEFAULT_STARTUP_TIMEOUT, self.wait_ready()).await;
        match ready {
            Ok(Ok(())) => {
                info!("OMP worker ready");
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(anyhow!(
                "OMP worker startup timed out after {:?}",
                DEFAULT_STARTUP_TIMEOUT
            )),
        }
    }

    /// Wait for worker to signal ready
    async fn wait_ready(&self) -> Result<()> {
        // Poll status until ready
        let mut attempts = 0;
        loop {
            let status = self.status.read().await;
            if status.ready {
                return Ok(());
            }
            drop(status);

            attempts += 1;
            if attempts > 100 {
                return Err(anyhow!("Worker failed to become ready"));
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Response reader task
    async fn response_reader(
        stdout: ChildStdout,
        pending: Arc<RwLock<std::collections::HashMap<u64, PendingResponse>>>,
        output_tx: mpsc::Sender<String>,
        status: Arc<RwLock<OmpWorkerStatus>>,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    debug!("Response reader shutting down");
                    break;
                }
                line = lines.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            // Try to parse as response
                            if let Ok(response) = serde_json::from_str::<OmpResponse>(&line) {
                                // Find and complete pending request
                                let mut pending_guard = pending.write().await;
                                if let Some(pending) = pending_guard.remove(&response.id) {
                                    let _ = pending.tx.send(Ok(response));
                                }
                            } else if let Ok(status_update) =
                                serde_json::from_str::<OmpWorkerStatus>(&line)
                            {
                                // Status update
                                let mut status_guard = status.write().await;
                                *status_guard = status_update;
                            } else {
                                // Unknown output - forward to output channel
                                let _ = output_tx.send(line).await;
                            }
                        }
                        Ok(None) => {
                            debug!("OMP worker stdout closed");
                            break;
                        }
                        Err(e) => {
                            error!("Error reading from OMP worker: {}", e);
                            break;
                        }
                    }
                }
            }
        }

        // Fail all pending requests
        let mut pending_guard = pending.write().await;
        for (_, pending) in pending_guard.drain() {
            let _ = pending.tx.send(Err(anyhow!("Worker process terminated")));
        }
    }

    /// Check for and clean up timed-out requests
    /// Returns the number of requests that timed out
    pub async fn cleanup_timed_out_requests(&mut self) -> usize {
        let mut pending_guard = self.pending.write().await;
        let timeout = self.config.response_timeout;
        let mut timed_out_count = 0;

        // Find all timed-out requests
        let timed_out_ids: Vec<u64> = pending_guard
            .iter()
            .filter(|(_, pending)| pending.is_timeout(timeout))
            .map(|(id, _)| *id)
            .collect();

        for id in timed_out_ids {
            if let Some(pending) = pending_guard.remove(&id) {
                // Get elapsed time before sending (to avoid borrow after move)
                let elapsed = pending.elapsed();
                // Send timeout error to the waiting caller
                let _ = pending.tx.send(Err(anyhow!("Request {} timed out after {:?}", id, elapsed)));
                timed_out_count += 1;
            }
        }

        if timed_out_count > 0 {
            info!("Cleaned up {} timed-out requests", timed_out_count);
        }

        timed_out_count
    }

    /// Get count of pending requests and their ages
    pub async fn get_pending_request_stats(&self) -> Vec<(u64, Duration)> {
        let pending_guard = self.pending.read().await;
        pending_guard
            .iter()
            .map(|(id, pending)| (*id, pending.elapsed()))
            .collect()
    }

    /// Send a request to the worker and wait for response
    pub async fn invoke(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<OmpResponse> {
        let stdin = self.stdin.as_mut().context("Worker not started")?;

        // Get next request ID
        let id = {
            let mut next_id = self.next_id.write().await;
            let id = *next_id;
            *next_id += 1;
            id
        };

        // Create request
        let request = OmpRequest {
            id,
            method: method.to_string(),
            params,
        };

        // Create response channel and pending tracker
        let (tx, rx) = oneshot::channel();
        let pending = PendingResponse::new(tx);

        // Register pending response
        {
            let mut pending_guard = self.pending.write().await;
            pending_guard.insert(id, pending);
        }

        // Send request
        let request_json = serde_json::to_string(&request)?;
        stdin.write_all(request_json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        // Clean up any timed-out requests before waiting for response
        let _ = self.cleanup_timed_out_requests().await;

        // Wait for response with timeout
        let response = timeout(self.config.response_timeout, rx)
            .await
            .context("Response timeout")?
            .context("Response channel closed")??;

        Ok(response)
    }

    /// Get worker status
    pub async fn status(&self) -> OmpWorkerStatus {
        self.status.read().await.clone()
    }

    /// Get output receiver
    pub fn take_output(&mut self) -> Option<mpsc::Receiver<String>> {
        self.output_rx.take()
    }

    /// Shutdown the worker
    pub async fn shutdown(&mut self) -> Result<()> {
        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Close stdin
        if let Some(mut stdin) = self.stdin.take() {
            let _ = stdin.shutdown().await;
        }

        // Kill process if still running
        if let Some(child) = self.child.take() {
            let mut child = child; // Move out
            let _ = child.kill().await;
        }

        info!("OMP worker shut down");
        Ok(())
    }
}

impl Drop for OmpWorker {
    fn drop(&mut self) {
        // Ensure process is killed on drop
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OmpWorkerConfig::default();
        assert!(config.tools.contains(&"python".to_string()));
        assert!(config.tools.contains(&"edit".to_string()));
        assert_eq!(config.response_timeout, DEFAULT_RESPONSE_TIMEOUT);
    }

    #[test]
    fn test_pending_response_timeout() {
        let (tx, _rx) = oneshot::channel();
        let pending = PendingResponse::new(tx);

        // Should not timeout immediately
        assert!(!pending.is_timeout(Duration::from_secs(10)));

        // Elapsed should be very small
        assert!(pending.elapsed() < Duration::from_millis(100));
    }
}
