//! Agent runner implementations
//!
//! Supports running agents via CLI, tmux, or directly.
//! Includes sandbox mode support using bubblewrap (bwrap) for isolation.
//! Includes LSP diagnostic validation after agent edits.

use crate::orchestrate::diagnostics::{validate_diagnostics_batch, EditTracker};
use crate::orchestrate::model::{AgentConfig, LspDiagnosticConfig, Task};
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;

/// Allowed tools for execution (security allowlist)
const ALLOWED_TOOLS: &[&str] = &[
    "claude", "gemini", "qwen", "opencode", "maestro", "pi",    // Pi-Mono CLI
    "amp",   // Amp CLI - first-class integration
    "codex", // Codex CLI - first-class integration
    "droid", // Droid CLI - first-class integration
];

/// Result of running an agent
#[derive(Debug, Clone)]
pub struct RunResult {
    pub success: bool,
    pub completed: bool,
    pub output: String,
    pub stderr: String,
    pub error_message: Option<String>,
    pub exit_code: Option<i32>,
}

/// Check if bubblewrap (bwrap) is available for sandbox mode
pub fn check_bwrap_available() -> Result<bool> {
    match Command::new("bwrap").arg("--version").output() {
        Ok(output) => Ok(output.status.success()),
        Err(_) => Ok(false),
    }
}

/// Build a bubblewrap wrapper command for sandbox isolation
///
/// This creates a minimal sandbox with:
/// - Read-only access to the working directory
/// - Temporary directory for writes
/// - Network access (required for most AI tools)
/// - Isolated process namespace
fn build_bwrap_wrapper(working_dir: &Path) -> Result<Vec<String>> {
    // Check if bwrap is available
    if !check_bwrap_available()? {
        return Err(anyhow!(
            "Sandbox mode requested but bubblewrap (bwrap) is not available. \
             Install bubblewrap or run without --sandbox flag."
        ));
    }

    let work_dir = working_dir
        .canonicalize()
        .context("Failed to canonicalize working directory")?;

    // Build bubblewrap arguments for minimal safe sandbox
    // Allow network access (required for AI tools)
    // Bind mount working directory as read-only (safe for analysis)
    // Provide a tmpfs for writes
    // CRITICAL: Bind /usr/bin for tool binary access
    let args = vec![
        "bwrap".to_string(),
        "--ro-bind".to_string(),
        work_dir.to_string_lossy().to_string(),
        work_dir.to_string_lossy().to_string(),
        "--bind".to_string(),
        "/usr".to_string(), // Bind /usr for tool binaries
        "--ro-bind".to_string(),
        "/bin".to_string(), // Bind /bin for shell
        "--ro-bind".to_string(),
        "/lib".to_string(), // Bind /lib for libraries
        "--ro-bind".to_string(),
        "/lib64".to_string(), // Bind /lib64 on some systems
        "--proc".to_string(),
        "/proc".to_string(), // Bind /proc for process info
        "--dev".to_string(),
        "/dev".to_string(), // Bind /dev for device access
        "--tmpfs".to_string(),
        "/tmp".to_string(),
        "--tmpfs".to_string(),
        "/home".to_string(),             // Isolated home directory
        "--unshare-all".to_string(),     // Unshare all namespaces (except net implied below)
        "--share-net".to_string(),       // Re-share network for API access
        "--die-with-parent".to_string(), // Exit when parent exits
        "--new-session".to_string(),     // Create new session
    ];

    Ok(args)
}

/// Agent runner trait
#[async_trait::async_trait]
pub trait DynAgentRunner: Send + Sync {
    async fn run(&self, prompt: &str, task: &Task) -> Result<RunResult>;
    fn is_available(&self) -> bool;
}

/// CLI-based agent runner (subprocess)
pub struct CliRunner {
    config: AgentConfig,
    working_dir: PathBuf,
    iteration_timeout_secs: u64,
    /// LSP diagnostic configuration
    lsp_diagnostics: LspDiagnosticConfig,
    /// Session ID for LSP communication
    session_id: Option<String>,
}

impl CliRunner {
    pub fn new(
        config: AgentConfig,
        working_dir: PathBuf,
        iteration_timeout_secs: u64,
    ) -> Self {
        Self {
            config,
            working_dir,
            iteration_timeout_secs,
            lsp_diagnostics: LspDiagnosticConfig::default(),
            session_id: None,
        }
    }

    /// Set LSP diagnostic configuration
    pub fn with_lsp_diagnostics(
        mut self,
        diagnostics: LspDiagnosticConfig,
    ) -> Self {
        self.lsp_diagnostics = diagnostics;
        self
    }

    /// Set session ID for LSP communication
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    async fn run_internal(&self, prompt: &str, task: &Task) -> Result<RunResult> {
        use tokio::io::{AsyncBufReadExt, BufReader};

        // Check sandbox availability if sandbox mode is enabled
        if self.config.sandbox && !check_bwrap_available()? {
            return Err(anyhow!(
                "Sandbox mode is enabled but bubblewrap (bwrap) is not available. \
                 Install bubblewrap: apt install bubblewrap (Debian/Ubuntu) or \
                 brew install bubblewrap (macOS with brew). Alternatively, run without --sandbox."
            ));
        }

        // Step 1: Capture file state before agent runs (for edit detection)
        let mut edit_tracker = if self.lsp_diagnostics.enabled {
            let mut tracker = EditTracker::new(self.working_dir.clone());
            // Ignore capture errors - non-critical for operation
            let _ = tracker.capture_before(&self.lsp_diagnostics);
            Some(tracker)
        } else {
            None
        };

        // Step 2: Create a unique secure prompt file using task ID and timestamp
        // This prevents race conditions when multiple orchestrate sessions run concurrently
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| anyhow!("Failed to get timestamp: {}", e))?
            .as_micros();
        let prompt_filename = format!(".maestro-prompt-{}-{}.txt", task.id, timestamp);
        let prompt_file = self.working_dir.join(&prompt_filename);
        tokio::fs::write(&prompt_file, prompt)
            .await
            .with_context(|| format!("Failed to write prompt file: {:?}", prompt_file))?;

        // Build command (with optional sandbox wrapper)
        let mut cmd = self.build_command_with_sandbox(&prompt_file)?;

        // Set working directory to track directory (critical for correct tool context)
        cmd.current_dir(&self.working_dir);

        // Set up output capture
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn process
        let mut child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn agent process: {}", self.config.tool))?;

        // Read stdout/stderr concurrently
        let stdout_task = if let Some(stdout) = child.stdout.take() {
            Some(tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = Vec::new();
                let mut reader_lines = reader.lines();
                while let Ok(Some(line)) = reader_lines.next_line().await {
                    lines.push(line);
                }
                lines.join("\n")
            }))
        } else {
            None
        };

        let stderr_task = if let Some(stderr) = child.stderr.take() {
            Some(tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = Vec::new();
                let mut reader_lines = reader.lines();
                while let Ok(Some(line)) = reader_lines.next_line().await {
                    lines.push(line);
                }
                lines.join("\n")
            }))
        } else {
            None
        };

        // Wait for completion with configured timeout (not hardcoded 300s)
        let output = timeout(
            Duration::from_secs(self.iteration_timeout_secs),
            child.wait(),
        )
        .await
        .map_err(|_| {
            anyhow!(
                "Agent execution timeout after {} seconds",
                self.iteration_timeout_secs
            )
        })?
        .context("Failed to wait for agent process")?;

        let stdout = match stdout_task {
            Some(handle) => handle.await.unwrap_or_else(|_| String::new()),
            None => String::new(),
        };

        let stderr = match stderr_task {
            Some(handle) => handle.await.unwrap_or_else(|_| String::new()),
            None => String::new(),
        };

        // Clean up prompt file
        let _ = tokio::fs::remove_file(&prompt_file).await;

        // Parse result
        let success = output.success();
        let completed = Self::detect_completion(&stdout);
        let error_message = if !success {
            if stderr.trim().is_empty() {
                Some(format!(
                    "Process failed with exit code: {:?}",
                    output.code()
                ))
            } else {
                Some(format!(
                    "Process failed with exit code: {:?}. stderr: {}",
                    output.code(),
                    stderr.trim()
                ))
            }
        } else {
            None
        };

        // Step 3: Detect edits and run LSP diagnostics if enabled
        let mut diagnostics_error = None;
        if let Some(ref mut tracker) = edit_tracker {
            if let Ok(modified_files) = tracker.detect_edits(&self.lsp_diagnostics) {
                if !modified_files.is_empty() {
                    tracing::info!(
                        "Detected {} modified files, running LSP diagnostics",
                        modified_files.len()
                    );

                    // Get session ID for LSP communication
                    let session_id = self.session_id.as_deref().unwrap_or("default");

                    // Run diagnostics on modified files
                    match validate_diagnostics_batch(
                        session_id,
                        &modified_files,
                        &self.lsp_diagnostics,
                    )
                    .await
                    {
                        Ok(validation) if !validation.passed => {
                            // Diagnostics failed
                            let diag_msg = validation.error_message.unwrap_or_else(|| {
                                format!(
                                    "LSP validation failed for {} file(s)",
                                    modified_files.len()
                                )
                            });

                            tracing::warn!("LSP diagnostics failed:\n{}", diag_msg);

                            // Mark as failed if errors are present
                            diagnostics_error = Some(diag_msg);

                            // Override success to force retry
                        }
                        Ok(_) => {
                            tracing::debug!(
                                "LSP diagnostics passed for {} file(s)",
                                modified_files.len()
                            );
                        }
                        Err(e) => {
                            // Don't fail on diagnostic errors, just log
                            tracing::warn!("LSP diagnostic check failed: {}", e);
                        }
                    }
                }
            }
        }

        // If diagnostics found errors, mark result as failed
        let final_success = if diagnostics_error.is_some() {
            false
        } else {
            success
        };

        let final_error_message = match (error_message, diagnostics_error) {
            (Some(proc_err), Some(diag_err)) => Some(format!("{}\n\n{}", proc_err, diag_err)),
            (Some(proc_err), None) => Some(proc_err),
            (None, Some(diag_err)) => Some(diag_err),
            (None, None) => None,
        };

        Ok(RunResult {
            success: final_success,
            completed,
            output: stdout,
            stderr,
            error_message: final_error_message,
            exit_code: output.code(),
        })
    }

    /// Build command for the configured tool with optional sandbox wrapper
    fn build_command_with_sandbox(&self, prompt_file: &Path) -> Result<TokioCommand> {
        // CRITICAL: Security check - verify tool is in allowlist unless dangerous_mode is enabled
        // This check must apply regardless of sandbox mode
        if !self.config.dangerous_mode && !ALLOWED_TOOLS.contains(&self.config.tool.as_str()) {
            return Err(anyhow!(
                "Tool '{}' is not in the allowlist. Allowed tools: {:?}. \
                 Set dangerous_mode=true to override (not recommended).",
                self.config.tool,
                ALLOWED_TOOLS
            ));
        }

        // Build the base tool arguments
        let tool_args = self.build_tool_args(prompt_file)?;

        if self.config.sandbox {
            // Wrap with bubblewrap for sandbox isolation
            let bwrap_args = build_bwrap_wrapper(&self.working_dir)?;

            // Create command with bwrap as the executable
            let mut cmd = TokioCommand::new(&bwrap_args[0]);
            // Add remaining bwrap arguments
            for arg in &bwrap_args[1..] {
                cmd.arg(arg);
            }
            // Add tool name and arguments after bwrap args
            cmd.arg(&self.config.tool);
            for arg in tool_args {
                cmd.arg(arg);
            }
            Ok(cmd)
        } else {
            // No sandbox, direct execution
            let mut cmd = TokioCommand::new(&self.config.tool);
            for arg in tool_args {
                cmd.arg(arg);
            }
            Ok(cmd)
        }
    }

    /// Build tool-specific arguments (without the tool name itself)
    fn build_tool_args(&self, prompt_file: &Path) -> Result<Vec<String>> {
        let prompt_str = prompt_file.to_string_lossy().to_string();

        let args = match self.config.tool.as_str() {
            "claude" => vec![prompt_str],
            "gemini" => vec!["chat".to_string(), "--prompt-file".to_string(), prompt_str],
            "qwen" => vec!["chat".to_string(), "-f".to_string(), prompt_str],
            "opencode" => vec!["chat".to_string(), "--prompt".to_string(), prompt_str],
            "pi" => vec![
                "-p".to_string(),
                "subagent".to_string(),
                "worker".to_string(),
                "--".to_string(),
                std::fs::read_to_string(prompt_file)?,
            ],
            _ => vec![prompt_str], // Default: just pass the prompt file
        };

        Ok(args)
    }

    fn detect_completion(output: &str) -> bool {
        // Check for completion promise
        if output.contains("<promise>COMPLETE</promise>") {
            return true;
        }

        // Check for specific completion markers
        output.contains("Task complete:") || output.contains("Implementation complete:")
    }
}

#[async_trait::async_trait]
impl DynAgentRunner for CliRunner {
    async fn run(&self, prompt: &str, task: &Task) -> Result<RunResult> {
        self.run_internal(prompt, task).await
    }

    fn is_available(&self) -> bool {
        Command::new(&self.config.tool)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Factory for creating runners
pub struct AgentRunnerFactory {
    config: AgentConfig,
    working_dir: PathBuf,
    iteration_timeout_secs: u64,
    /// LSP diagnostic configuration
    lsp_diagnostics: LspDiagnosticConfig,
    /// Session ID for LSP communication
    session_id: Option<String>,
}

impl AgentRunnerFactory {
    pub fn new(config: AgentConfig, working_dir: PathBuf, iteration_timeout_secs: u64) -> Self {
        Self {
            config,
            working_dir,
            iteration_timeout_secs,
            lsp_diagnostics: LspDiagnosticConfig::default(),
            session_id: None,
        }
    }

    /// Set LSP diagnostic configuration
    pub fn with_lsp_diagnostics(mut self, diagnostics: LspDiagnosticConfig) -> Self {
        self.lsp_diagnostics = diagnostics;
        self
    }

    /// Set session ID for LSP communication
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn create_runner(&self) -> CliRunner {
        CliRunner::new(
            self.config.clone(),
            self.working_dir.clone(),
            self.iteration_timeout_secs,
        )
        .with_lsp_diagnostics(self.lsp_diagnostics.clone())
        .with_session_id(
            self.session_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        )
    }
}

/// Convenience wrapper for the runner
pub struct AgentRunner {
    runner: CliRunner,
}

impl AgentRunner {
    pub fn new(config: AgentConfig, working_dir: PathBuf, iteration_timeout_secs: u64) -> Self {
        Self {
            runner: CliRunner::new(config, working_dir, iteration_timeout_secs),
        }
    }

    pub async fn run(&self, prompt: &str, task: &Task) -> Result<RunResult> {
        self.runner.run(prompt, task).await
    }

    pub fn is_available(&self) -> bool {
        self.runner.is_available()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_detection() {
        assert!(CliRunner::detect_completion(
            "Some output\n<promise>COMPLETE</promise>\n"
        ));
        assert!(CliRunner::detect_completion("Task complete: done\n"));
        assert!(!CliRunner::detect_completion("Some output without promise"));
    }
}
