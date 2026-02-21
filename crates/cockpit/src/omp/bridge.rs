//! OMP IPC Bridge
//!
//! High-level interface for invoking OMP tools from Maestro.

use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::protocol::{OmpError, OmpToolResult, OmpWorkerStatus};
use super::worker::{OmpWorker, OmpWorkerConfig};

/// OMP tool names that can be invoked
pub const TOOL_PYTHON: &str = "python";
pub const TOOL_EDIT: &str = "edit";
pub const TOOL_GREP: &str = "grep";
pub const TOOL_FIND: &str = "find";
pub const TOOL_READ: &str = "read";
pub const TOOL_WRITE: &str = "write";

/// All available OMP tools
pub const ALL_TOOLS: &[&str] = &[
    TOOL_PYTHON,
    TOOL_EDIT,
    TOOL_GREP,
    TOOL_FIND,
    TOOL_READ,
    TOOL_WRITE,
];

/// OMP bridge for invoking tools from Maestro
pub struct OmpBridge {
    /// Worker instance (lazy initialized)
    worker: Arc<RwLock<Option<OmpWorker>>>,
    /// Worker configuration
    config: OmpWorkerConfig,
}

impl OmpBridge {
    /// Create a new OMP bridge
    pub fn new(config: OmpWorkerConfig) -> Self {
        Self {
            worker: Arc::new(RwLock::new(None)),
            config,
        }
    }

    /// Ensure worker is started
    async fn ensure_worker(&self) -> Result<()> {
        let mut worker = self.worker.write().await;

        if worker.is_none() {
            info!("Starting OMP worker on demand");
            let mut w = OmpWorker::new(self.config.clone());
            w.start().await.context("Failed to start OMP worker")?;
            *worker = Some(w);
        }

        Ok(())
    }

    /// Invoke a tool
    pub async fn invoke(&self, tool: &str, params: serde_json::Value) -> Result<OmpToolResult> {
        // Validate tool name
        if !ALL_TOOLS.contains(&tool) {
            return Err(anyhow!(OmpError::tool_not_found(tool).to_string()));
        }

        self.ensure_worker().await?;

        let mut worker = self.worker.write().await;
        let worker = worker
            .as_mut()
            .ok_or_else(|| anyhow!(OmpError::worker_not_ready().to_string()))?;

        let response = worker
            .invoke(
                "invoke_tool",
                serde_json::json!({ "tool": tool, "params": params }),
            )
            .await
            .context("Failed to invoke tool")?;

        // Parse result
        if let Some(error) = response.error {
            return Err(anyhow!(OmpError::new(error.code, error.message).to_string()));
        }

        let result = response
            .result
            .ok_or_else(|| anyhow!("No result in response"))?;

        let tool_result: OmpToolResult =
            serde_json::from_value(result).context("Failed to parse tool result")?;

        Ok(tool_result)
    }

    /// Get worker status
    pub async fn status(&self) -> OmpWorkerStatus {
        let worker = self.worker.read().await;

        match worker.as_ref() {
            Some(w) => w.status().await,
            None => OmpWorkerStatus::uninitialized(),
        }
    }

    /// Shutdown worker
    pub async fn shutdown(&self) -> Result<()> {
        let mut worker = self.worker.write().await;

        if let Some(w) = worker.take() {
            let mut w = w; // Move out of option
            w.shutdown().await?;
        }

        Ok(())
    }

    /// Check if OMP is available
    pub fn is_available(&self) -> bool {
        // Check if OMP path exists and has required files
        let worker_path = self
            .config
            .omp_path
            .join("packages/coding-agent/src/worker.ts");
        worker_path.exists()
    }

    /// Execute Python code
    pub async fn execute_python(&self, code: &str, cwd: Option<&PathBuf>) -> Result<String> {
        let params = serde_json::json!({
            "code": code,
            "cwd": cwd.map(|p| p.to_string_lossy().to_string()),
        });

        let result = self.invoke(TOOL_PYTHON, params).await?;

        if !result.success {
            return Err(anyhow!(
                "Python execution failed: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        Ok(result.output)
    }

    /// Apply a patch edit
    pub async fn apply_edit(&self, file_path: &str, diff: &str) -> Result<bool> {
        let params = serde_json::json!({
            "path": file_path,
            "diff": diff,
        });

        let result = self.invoke(TOOL_EDIT, params).await?;

        if !result.success {
            return Err(anyhow!(
                "Edit failed: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        Ok(result.success)
    }

    /// Search with ripgrep WASM
    pub async fn grep(
        &self,
        pattern: &str,
        path: &str,
        options: Option<serde_json::Value>,
    ) -> Result<String> {
        let params = serde_json::json!({
            "pattern": pattern,
            "path": path,
            "options": options,
        });

        let result = self.invoke(TOOL_GREP, params).await?;

        if !result.success {
            return Err(anyhow!(
                "Grep failed: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        Ok(result.output)
    }

    /// Find files with glob
    pub async fn find(&self, pattern: &str, path: &str) -> Result<String> {
        let params = serde_json::json!({
            "pattern": pattern,
            "path": path,
        });

        let result = self.invoke(TOOL_FIND, params).await?;

        if !result.success {
            return Err(anyhow!(
                "Find failed: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        Ok(result.output)
    }

    /// Read a file
    pub async fn read(
        &self,
        path: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<String> {
        let params = serde_json::json!({
            "path": path,
            "offset": offset,
            "limit": limit,
        });

        let result = self.invoke(TOOL_READ, params).await?;

        if !result.success {
            return Err(anyhow!(
                "Read failed: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        Ok(result.output)
    }

    /// Write to a file
    pub async fn write(&self, path: &str, content: &str) -> Result<bool> {
        let params = serde_json::json!({
            "path": path,
            "content": content,
        });

        let result = self.invoke(TOOL_WRITE, params).await?;

        if !result.success {
            return Err(anyhow!(
                "Write failed: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        Ok(result.success)
    }
}

/// Global OMP bridge instance (lazy initialized)
static OMP_BRIDGE: tokio::sync::OnceCell<Arc<OmpBridge>> = tokio::sync::OnceCell::const_new();

/// Get or create the global OMP bridge
pub async fn get_omp_bridge(config: Option<OmpWorkerConfig>) -> Result<Arc<OmpBridge>> {
    OMP_BRIDGE
        .get_or_try_init(|| async {
            let config = config.unwrap_or_else(|| OmpWorkerConfig {
                omp_path: PathBuf::from("vendor/oh-my-pi"),
                ..Default::default()
            });
            let bridge = Arc::new(OmpBridge::new(config));
            Ok(bridge)
        })
        .await
        .cloned()
}

/// Check if OMP is available globally
///
/// Uses PATH-first discovery with no hardcoded absolute paths.
/// Checks:
/// 1. vendor/oh-my-pi directory (relative to cwd)
/// 2. 'pi' or 'omp' in PATH
/// 3. User-local installation (~/.local/bin, ~/.cargo/bin)
/// 4. oh-my-pi directory in common locations
pub fn is_omp_available() -> bool {
    // Check 1: vendor/oh-my-pi/packages/coding-agent/src/worker.ts (relative path)
    let worker_path = PathBuf::from("vendor/oh-my-pi/packages/coding-agent/src/worker.ts");
    if worker_path.exists() {
        return true;
    }

    // Check 2: Check if 'pi' or 'omp' command exists in PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            if dir.is_empty() {
                continue;
            }
            let pi_path = PathBuf::from(dir).join("pi");
            let omp_path = PathBuf::from(dir).join("omp");
            if pi_path.exists() || omp_path.exists() {
                return true;
            }
        }
    }

    // Check 3: User-local installation paths (computed from home, no hardcoded paths)
    if let Some(home) = dirs::home_dir() {
        let user_local_paths = vec![
            home.join(".local/bin/pi"),
            home.join(".local/bin/omp"),
            home.join(".cargo/bin/pi"),
            home.join(".cargo/bin/omp"),
            home.join("bin/pi"),
            home.join("bin/omp"),
        ];

        for path in user_local_paths {
            if path.exists() {
                return true;
            }
        }

        // Check 4: Check for oh-my-pi directory in user locations
        let omp_dir_paths = vec![
            home.join("oh-my-pi"),
            home.join(".oh-my-pi"),
        ];

        for omp_dir in omp_dir_paths {
            let worker = omp_dir.join("packages/coding-agent/src/worker.ts");
            if worker.exists() {
                return true;
            }
        }
    }

    // Check 5: Relative path for development
    let dev_omp = PathBuf::from("..").join("oh-my-pi");
    let worker = dev_omp.join("packages/coding-agent/src/worker.ts");
    if worker.exists() {
        return true;
    }

    false
}
