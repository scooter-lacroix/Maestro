//! Agent runner implementations
//!
//! Supports running agents via CLI, tmux, or directly.

use crate::orchestrate::model::{AgentConfig, Task};
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;

/// Allowed tools for execution (security allowlist)
const ALLOWED_TOOLS: &[&str] = &[
    "claude",
    "gemini",
    "qwen",
    "opencode",
    "maestro",
    "amp",       // Amp CLI - first-class integration
    "codex",     // Codex CLI - first-class integration
    "droid",     // Droid CLI - first-class integration
];

/// Result of running an agent
#[derive(Debug, Clone)]
pub struct RunResult {
    pub success: bool,
    pub completed: bool,
    pub output: String,
    pub error_message: Option<String>,
    pub exit_code: Option<i32>,
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
}

impl CliRunner {
    pub fn new(config: AgentConfig, working_dir: PathBuf, iteration_timeout_secs: u64) -> Self {
        Self { config, working_dir, iteration_timeout_secs }
    }

    /// Build command for the configured tool
    fn build_command(&self, prompt_file: &Path) -> Result<TokioCommand> {
        // Security check: verify tool is in allowlist unless dangerous_mode is enabled
        if !self.config.dangerous_mode && !ALLOWED_TOOLS.contains(&self.config.tool.as_str()) {
            return Err(anyhow!(
                "Tool '{}' is not in the allowlist. Allowed tools: {:?}. \
                 Set dangerous_mode=true to override (not recommended).",
                self.config.tool, ALLOWED_TOOLS
            ));
        }

        let cmd = match self.config.tool.as_str() {
            "claude" => {
                let mut c = TokioCommand::new("claude");
                c.arg(prompt_file);
                c
            }
            "gemini" => {
                let mut c = TokioCommand::new("gemini");
                c.arg("chat");
                c.arg("--prompt-file");
                c.arg(prompt_file);
                c
            }
            "qwen" => {
                let mut c = TokioCommand::new("qwen");
                c.arg("chat");
                c.arg("-f");
                c.arg(prompt_file);
                c
            }
            "opencode" => {
                let mut c = TokioCommand::new("opencode");
                c.arg("chat");
                c.arg("--prompt");
                c.arg(prompt_file);
                c
            }
            tool => {
                let mut c = TokioCommand::new(tool);
                c.arg(prompt_file);
                c
            }
        };

        Ok(cmd)
    }

    async fn run_internal(&self, prompt: &str, task: &Task) -> Result<RunResult> {
        use tokio::io::{AsyncBufReadExt, BufReader};

        // Create a unique secure prompt file using task ID and timestamp
        // This prevents race conditions when multiple orchestrate sessions run concurrently
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| anyhow!("Failed to get timestamp: {}", e))?
            .as_micros();
        let prompt_filename = format!(".maestro-prompt-{}-{}.txt", task.id, timestamp);
        let prompt_file = self.working_dir.join(&prompt_filename);
        tokio::fs::write(&prompt_file, prompt).await
            .with_context(|| format!("Failed to write prompt file: {:?}", prompt_file))?;

        // Build command (returns Result for security check)
        let mut cmd = self.build_command(&prompt_file)?;

        // Set working directory to track directory (critical for correct tool context)
        cmd.current_dir(&self.working_dir);

        // Set up output capture
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn process
        let mut child = cmd.spawn()
            .with_context(|| format!("Failed to spawn agent process: {}", self.config.tool))?;

        // Read stdout
        let stdout = if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = Vec::new();
            let mut reader_lines = reader.lines();
            while let Ok(Some(line)) = reader_lines.next_line().await {
                lines.push(line);
            }
            lines.join("\n")
        } else {
            String::new()
        };

        // Wait for completion with configured timeout (not hardcoded 300s)
        let output = timeout(Duration::from_secs(self.iteration_timeout_secs), child.wait()).await
            .map_err(|_| anyhow!("Agent execution timeout after {} seconds", self.iteration_timeout_secs))?
            .context("Failed to wait for agent process")?;

        // Clean up prompt file
        let _ = tokio::fs::remove_file(&prompt_file).await;

        // Parse result
        let success = output.success();
        let completed = Self::detect_completion(&stdout);

        Ok(RunResult {
            success,
            completed,
            output: stdout,
            error_message: if !success {
                Some(format!("Process failed with exit code: {:?}", output.code()))
            } else {
                None
            },
            exit_code: output.code(),
        })
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
}

impl AgentRunnerFactory {
    pub fn new(config: AgentConfig, working_dir: PathBuf, iteration_timeout_secs: u64) -> Self {
        Self { config, working_dir, iteration_timeout_secs }
    }

    pub fn create_runner(&self) -> CliRunner {
        CliRunner::new(self.config.clone(), self.working_dir.clone(), self.iteration_timeout_secs)
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
        assert!(CliRunner::detect_completion("Some output\n<promise>COMPLETE</promise>\n"));
        assert!(CliRunner::detect_completion("Task complete: done\n"));
        assert!(!CliRunner::detect_completion("Some output without promise"));
    }
}
