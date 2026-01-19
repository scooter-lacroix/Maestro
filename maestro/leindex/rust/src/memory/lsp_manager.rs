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
//! use maestro_leindex_analyzers::memory::LspManager;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let manager = LspManager::new();
//!     // Start LSP for a session
//!     manager.start_lsp("session-123", maestro_leindex_analyzers::memory::LspType::Rust).await?;
//!     Ok(())
//! }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command as TokioCommand;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::turso_backend::{LspServerState, LspStatus, TursoStorageBackend};

/// LSP server types supported by Maestro
///
/// Each variant represents a specific language server that can be spawned on-demand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            auto_start: true,
            binary_path: None,
            additional_args: Vec::new(),
            env_vars: HashMap::new(),
        }
    }
}

/// LSP process tracking information
///
/// Tracks runtime information about a running LSP server process.
#[derive(Debug, Clone)]
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
    /// When the process was started
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last error message (if any)
    pub last_error: Option<String>,
}

impl LspProcess {
    /// Create a new LSP process tracking record
    pub fn new(lsp_type: LspType, session_id: String) -> Self {
        Self {
            lsp_type,
            session_id,
            pid: None,
            status: LspStatus::Stopped,
            port: None,
            started_at: None,
            last_error: None,
        }
    }

    /// Convert to database state for persistence
    pub fn to_db_state(&self) -> LspServerState {
        LspServerState {
            id: 0, // Will be set by database
            session_id: self.session_id.clone(),
            language: self.lsp_type.language().to_string(),
            lsp_name: self.lsp_type.binary_name().to_string(),
            status: self.status.as_str().to_string(),
            pid: self.pid.map(|p| p as i64),
            port: self.port.map(|p| p as i64),
            auto_start: true,
            last_started: self.started_at.map(|d| d.to_rfc3339()),
            last_error: self.last_error.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: Some(chrono::Utc::now().to_rfc3339()),
        }
    }
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
        Self {
            storage,
            running_lsps: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create LSP manager with default storage
    ///
    /// Uses default Turso database location.
    pub async fn with_default_storage() -> Result<Self> {
        let storage = TursoStorageBackend::new(None).await?;
        Ok(Self::new(storage))
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
    /// Returns `Ok(())` if LSP started successfully, or error with details
    ///
    /// ## Graceful Degradation
    ///
    /// If the LSP binary is not found or fails to start, this method:
    /// - Logs the error
    /// - Updates status to Error
    /// - Returns Ok(()) to allow continuing without the LSP
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
                    debug!("LSP '{}' already running for session '{}'", lsp_type.display_name(), session_id);
                    return Ok(());
                }
            }
        }

        // Update status to starting
        {
            let mut running = self.running_lsps.write().await;
            let mut process = LspProcess::new(lsp_type, session_id.to_string());
            process.status = LspStatus::Starting;
            running.insert(lsp_key.clone(), process);
        }

        // Spawn the LSP process
        let binary = config
            .binary_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(lsp_type.binary_name()));

        debug!("Spawning LSP process: {:?}", binary);

        let result = TokioCommand::new(&binary)
            .args(&config.additional_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&config.env_vars)
            .spawn();

        match result {
            Ok(child) => {
                let pid = child.id();
                info!(
                    "LSP '{}' started successfully (PID: {})",
                    lsp_type.display_name(),
                    pid.unwrap_or(0)
                );

                // Update status to running
                {
                    let mut running = self.running_lsps.write().await;
                    let process = running.get_mut(&lsp_key).unwrap();
                    process.pid = pid;
                    process.status = LspStatus::Running;
                    process.started_at = Some(chrono::Utc::now());
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

                // Don't return error - graceful degradation
                Ok(())
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

        let mut running = self.running_lsps.write().await;

        if let Some(mut process) = running.remove(&lsp_key) {
            // TODO: Actually kill the process
            // For now, just remove from tracking
            process.status = LspStatus::Stopped;
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

    /// Persist LSP state to database
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_type`: Type of LSP to persist
    async fn persist_lsp_state(&self, session_id: &str, lsp_type: LspType) -> Result<()> {
        let lsp_key = format!("{}:{}", session_id, lsp_type.binary_name());

        let running = self.running_lsps.read().await;
        if let Some(process) = running.get(&lsp_key) {
            let state = process.to_db_state();

            // TODO: Actually persist to database
            // For now, just log
            debug!(
                "Would persist LSP state: {} (status: {})",
                state.lsp_name, state.status
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_type_properties() {
        assert_eq!(LspType::Rust.binary_name(), "rust-analyzer");
        assert_eq!(LspType::Python.binary_name(), "ruff-lsp");
        assert_eq!(LspType::TypeScript.binary_name(), "typescript-language-server");

        assert_eq!(LspType::Rust.language(), "rust");
        assert_eq!(LspType::Python.language(), "python");
        assert_eq!(LspType::TypeScript.language(), "typescript");

        assert_eq!(LspType::Rust.file_extensions(), &["rs"]);
        assert_eq!(LspType::Python.file_extensions(), &["py"]);
        assert_eq!(LspType::TypeScript.file_extensions(), &["ts", "tsx", "js", "jsx"]);
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
        let process = LspProcess::new(LspType::Rust, "test-session".to_string());
        assert_eq!(process.lsp_type, LspType::Rust);
        assert_eq!(process.session_id, "test-session");
        assert_eq!(process.status, LspStatus::Stopped);
        assert!(process.pid.is_none());
        assert!(process.last_error.is_none());
    }

    #[tokio::test]
    async fn test_lsp_manager_creation() {
        let storage = TursoStorageBackend::in_memory()
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
}
