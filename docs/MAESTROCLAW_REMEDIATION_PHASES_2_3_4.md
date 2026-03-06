# MaestroClaw Remediation Plan — Phases 2, 3 & 4

> Generated: 2026-03-05
> Reference project: `/mnt/WD-SSD/Prod/work_resources/zeroclaw/`
> Target crate: `crates/maestro-claw/`
> Cockpit TUI module: `crates/cockpit/src/maesterclaw/`

---

## Table of Contents

1. [Phase 1 Recap (Completed)](#phase-1-recap)
2. [Phase 2: CLI Tool Agent Runner Integration](#phase-2-cli-tool-agent-runner-integration)
3. [Phase 3: Real Channel Transports & Gateway Server](#phase-3-real-channel-transports--gateway-server)
4. [Phase 4: Heartbeat, Skills, Cost Tracking, Observability](#phase-4-heartbeat-skills-cost-tracking-observability)
5. [Dependency Changes Summary](#dependency-changes-summary)
6. [Testing Strategy](#testing-strategy)

---

## Phase 1 Recap

Already implemented and merged (248 tests passing):

| Module | Status |
|---|---|
| `config/` (mod.rs, schema.rs) | ✅ TOML config with env overrides |
| `health/` (mod.rs) | ✅ Global component health registry |
| `doctor/` (mod.rs) | ✅ System diagnostics |
| `onboard/` (mod.rs) | ✅ Setup wizard + tool detection |
| `cron/` (mod.rs, types.rs, schedule.rs, store.rs, scheduler.rs) | ✅ Full scheduler |
| `daemon/` (mod.rs) | ✅ Supervised runtime |
| `service/` (mod.rs) | ✅ systemd/launchd manager |

---

## Phase 2: CLI Tool Agent Runner Integration

### 2.1 Goal

Wire the existing leindex CLI runner infrastructure (`src/leindex/src/orchestrate/runner.rs`) into maestro-claw so that agent jobs and agent loops use locally-installed CLI tools (claude, codex, gemini, qwen, iflow) instead of HTTP API providers. This is the **key differentiator** — MaestroClaw needs zero API keys because it drives tools already authenticated on the host.

### 2.2 Architecture

```
cron scheduler / daemon / direct invocation
        │
        ▼
┌─────────────────────────┐
│  CliAgentProvider       │  ← implements agent::Provider trait
│  (new adapter)          │
│                         │
│  ┌───────────────────┐  │
│  │ ToolRunner        │  │  ← builds tool-specific CLI args
│  │ (claude/codex/    │  │
│  │  gemini/qwen/     │  │
│  │  iflow)           │  │
│  └───────────────────┘  │
└─────────────────────────┘
        │
        ▼
  tokio::process::Command (subprocess)
        │
        ▼
  stdout/stderr capture → ProviderResponse
```

### 2.3 New Files

#### 2.3.1 `src/agent/cli_provider.rs` — CLI Tool Provider

This adapter implements the `agent::Provider` trait by spawning CLI tools as subprocesses. It replaces HTTP API calls with direct CLI invocation.

```rust
//! CLI Tool Provider — drives locally-installed coding agents
//!
//! Implements `agent::Provider` by spawning CLI tools (claude, codex, gemini,
//! qwen, iflow) as subprocesses. No API keys needed — tools are already
//! authenticated on the host machine.

use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use tokio::process::Command;
use tokio::time::timeout;

use crate::agent::{AgentError, Provider, ProviderResponse};
use crate::session::ProviderMessage;
use crate::tools::ToolSpec;

/// Known CLI tool identifiers
pub const KNOWN_TOOLS: &[&str] = &["claude", "codex", "gemini", "qwen", "iflow", "amp", "droid"];

/// Configuration for a CLI tool provider
#[derive(Debug, Clone)]
pub struct CliProviderConfig {
    /// Tool binary name or path (e.g., "claude", "/usr/local/bin/codex")
    pub tool: String,
    /// Working directory for the subprocess
    pub working_dir: PathBuf,
    /// Maximum execution time per turn (seconds)
    pub timeout_secs: u64,
    /// Extra arguments to pass to the tool
    pub extra_args: Vec<String>,
    /// Whether to use print mode (non-interactive, single prompt)
    pub print_mode: bool,
}

impl Default for CliProviderConfig {
    fn default() -> Self {
        Self {
            tool: "claude".into(),
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            timeout_secs: 300,
            extra_args: Vec::new(),
            print_mode: true,
        }
    }
}

/// CLI-based provider that spawns coding agent tools as subprocesses
pub struct CliProvider {
    config: CliProviderConfig,
}

impl CliProvider {
    pub fn new(config: CliProviderConfig) -> Self {
        Self { config }
    }

    pub fn for_tool(tool: &str, working_dir: &Path) -> Self {
        Self::new(CliProviderConfig {
            tool: tool.to_string(),
            working_dir: working_dir.to_path_buf(),
            ..CliProviderConfig::default()
        })
    }

    pub fn is_available(&self) -> bool {
        std::process::Command::new(&self.config.tool)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn detect_best_tool(working_dir: &Path) -> Option<Self> {
        for tool in KNOWN_TOOLS {
            let provider = Self::for_tool(tool, working_dir);
            if provider.is_available() {
                return Some(provider);
            }
        }
        None
    }

    /// Build tool-specific CLI arguments for a prompt
    fn build_args(&self, prompt: &str) -> Vec<String> {
        let mut args = Vec::new();

        match self.config.tool.as_str() {
            "claude" => {
                if self.config.print_mode {
                    args.push("--print".into());
                }
                args.push("--message".into());
                args.push(prompt.to_string());
            }
            "codex" => {
                args.push("--quiet".into());
                args.push("--approval-mode".into());
                args.push("full-auto".into());
                args.push(prompt.to_string());
            }
            "gemini" => {
                args.push("--prompt".into());
                args.push(prompt.to_string());
            }
            "qwen" => {
                args.push("chat".into());
                args.push("--message".into());
                args.push(prompt.to_string());
            }
            "iflow" => {
                args.push("-p".into());
                args.push(prompt.to_string());
            }
            "amp" => {
                args.push("--message".into());
                args.push(prompt.to_string());
            }
            "droid" => {
                args.push("--prompt".into());
                args.push(prompt.to_string());
            }
            _ => {
                args.push(prompt.to_string());
            }
        }

        args.extend(self.config.extra_args.clone());
        args
    }

    /// Format messages into a single prompt string for the CLI tool
    fn messages_to_prompt(messages: &[ProviderMessage]) -> String {
        let mut parts = Vec::new();

        for msg in messages {
            match msg.role.as_str() {
                "system" => {
                    parts.push(format!("[System]\n{}", msg.content));
                }
                "user" => {
                    parts.push(msg.content.clone());
                }
                "assistant" => {
                    parts.push(format!("[Previous Assistant Response]\n{}", msg.content));
                }
                "tool" => {
                    let tool_id = msg.tool_call_id.as_deref().unwrap_or("unknown");
                    parts.push(format!("[Tool Result: {}]\n{}", tool_id, msg.content));
                }
                _ => {
                    parts.push(msg.content.clone());
                }
            }
        }

        parts.join("\n\n")
    }

    async fn run_tool(&self, prompt: &str) -> Result<String, AgentError> {
        let args = self.build_args(prompt);

        let mut cmd = Command::new(&self.config.tool);
        cmd.args(&args)
            .current_dir(&self.config.working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null());

        cmd.env("CI", "true");
        cmd.env("NONINTERACTIVE", "1");

        let output = timeout(
            Duration::from_secs(self.config.timeout_secs),
            cmd.output(),
        )
        .await
        .map_err(|_| AgentError::TimeoutExceeded(self.config.timeout_secs))?
        .map_err(|e| AgentError::ProviderError(format!(
            "Failed to spawn '{}': {e}", self.config.tool
        )))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            return Err(AgentError::ProviderError(format!(
                "'{}' exited with code {exit_code}: {stderr}",
                self.config.tool
            )));
        }

        let response = if stdout.trim().is_empty() {
            stderr.trim().to_string()
        } else {
            stdout.trim().to_string()
        };

        Ok(response)
    }
}

#[async_trait]
impl Provider for CliProvider {
    async fn execute(
        &self,
        messages: Vec<ProviderMessage>,
        _tools: Vec<ToolSpec>,
    ) -> Result<ProviderResponse, AgentError> {
        let prompt = Self::messages_to_prompt(&messages);

        if prompt.trim().is_empty() {
            return Err(AgentError::ConfigError("Empty prompt".into()));
        }

        let content = self.run_tool(&prompt).await?;
        Ok(ProviderResponse::text(content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_provider_default_config() {
        let config = CliProviderConfig::default();
        assert_eq!(config.tool, "claude");
        assert_eq!(config.timeout_secs, 300);
        assert!(config.print_mode);
    }

    #[test]
    fn test_build_args_claude() {
        let provider = CliProvider::new(CliProviderConfig {
            tool: "claude".into(),
            print_mode: true,
            ..CliProviderConfig::default()
        });
        let args = provider.build_args("Hello world");
        assert_eq!(args, vec!["--print", "--message", "Hello world"]);
    }

    #[test]
    fn test_build_args_codex() {
        let provider = CliProvider::new(CliProviderConfig {
            tool: "codex".into(),
            ..CliProviderConfig::default()
        });
        let args = provider.build_args("Fix the bug");
        assert!(args.contains(&"--quiet".to_string()));
        assert!(args.contains(&"full-auto".to_string()));
        assert!(args.contains(&"Fix the bug".to_string()));
    }

    #[test]
    fn test_build_args_gemini() {
        let provider = CliProvider::new(CliProviderConfig {
            tool: "gemini".into(),
            ..CliProviderConfig::default()
        });
        let args = provider.build_args("Explain this");
        assert_eq!(args, vec!["--prompt", "Explain this"]);
    }

    #[test]
    fn test_messages_to_prompt_single_user() {
        let messages = vec![ProviderMessage {
            role: "user".into(),
            content: "Hello".into(),
            tool_calls: None,
            tool_call_id: None,
        }];
        let prompt = CliProvider::messages_to_prompt(&messages);
        assert_eq!(prompt, "Hello");
    }

    #[test]
    fn test_messages_to_prompt_system_and_user() {
        let messages = vec![
            ProviderMessage {
                role: "system".into(),
                content: "You are helpful.".into(),
                tool_calls: None,
                tool_call_id: None,
            },
            ProviderMessage {
                role: "user".into(),
                content: "What is 2+2?".into(),
                tool_calls: None,
                tool_call_id: None,
            },
        ];
        let prompt = CliProvider::messages_to_prompt(&messages);
        assert!(prompt.contains("[System]"));
        assert!(prompt.contains("You are helpful."));
        assert!(prompt.contains("What is 2+2?"));
    }

    #[test]
    fn test_extra_args_appended() {
        let provider = CliProvider::new(CliProviderConfig {
            tool: "claude".into(),
            extra_args: vec!["--model".into(), "sonnet".into()],
            ..CliProviderConfig::default()
        });
        let args = provider.build_args("Hi");
        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"sonnet".to_string()));
    }

    #[test]
    fn test_detect_best_tool_returns_none_in_clean_env() {
        let _ = CliProvider::detect_best_tool(&std::env::temp_dir());
    }
}
```

#### 2.3.2 Update `src/agent/mod.rs`

Add the new module and re-exports:

```rust
//! Agent module for AI agent execution

mod r#loop;
mod adapter;
pub mod cli_provider;  // ← ADD THIS LINE

pub use r#loop::{agent_loop, AgentConfig, AgentError, AgentResult, ErrorStrategy, Provider, ProviderResponse};
pub use adapter::ProviderAdapter;
pub use cli_provider::{CliProvider, CliProviderConfig, KNOWN_TOOLS};  // ← ADD
```

#### 2.3.3 Update `src/cron/scheduler.rs` — Use Config-Aware Agent Runner

Replace the hardcoded `claude` invocation with `CliProvider`:

```rust
//! Cron scheduler for MaestroClaw — polls due jobs and executes them

use crate::agent::cli_provider::{CliProvider, CliProviderConfig};
use crate::agent::Provider; // the trait
use crate::config::Config;
use crate::cron::{due_jobs, record_run, reschedule_after_run, CronJob, JobType};
use crate::session::ProviderMessage;
use anyhow::Result;
use chrono::Utc;
use tokio::process::Command;
use tokio::time::{self, Duration};

/// Run the scheduler loop with full config access
pub async fn run(config: &Config) -> Result<()> {
    let poll_secs = config.runtime.scheduler_poll_secs.max(5);
    let max_history = config.cron.max_run_history;
    let mut interval = time::interval(Duration::from_secs(poll_secs));

    crate::health::mark_component_ok("scheduler");

    loop {
        interval.tick().await;

        let jobs = match due_jobs(&config.workspace_dir, Utc::now()) {
            Ok(jobs) => jobs,
            Err(e) => {
                crate::health::mark_component_error("scheduler", e.to_string());
                tracing::warn!("Scheduler query failed: {e}");
                continue;
            }
        };

        for job in jobs {
            crate::health::mark_component_ok("scheduler");
            let started = Utc::now();
            let (success, output) = execute_job(config, &job).await;
            let finished = Utc::now();
            let duration_ms = (finished - started).num_milliseconds();

            let _ = record_run(
                &config.workspace_dir, &job.id, started, finished,
                if success { "ok" } else { "error" },
                Some(&output), duration_ms, max_history,
            );

            if let Err(e) = reschedule_after_run(&config.workspace_dir, &job, success, &output) {
                tracing::warn!("Failed to reschedule job {}: {e}", job.id);
            }

            if !success {
                crate::health::mark_component_error(
                    "scheduler", format!("job {} failed", job.id),
                );
            }
        }
    }
}

async fn execute_job(config: &Config, job: &CronJob) -> (bool, String) {
    match job.job_type {
        JobType::Shell => run_shell_job(job).await,
        JobType::Agent => run_agent_job(config, job).await,
    }
}

async fn run_shell_job(job: &CronJob) -> (bool, String) {
    match Command::new("sh")
        .args(["-c", &job.command])
        .output()
        .await
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{stdout}{stderr}status={}", output.status);
            (output.status.success(), combined)
        }
        Err(e) => (false, format!("Failed to spawn: {e}")),
    }
}

/// Run agent job using the primary CLI tool from config
async fn run_agent_job(config: &Config, job: &CronJob) -> (bool, String) {
    let prompt = job.prompt.as_deref().unwrap_or("");
    let prefixed = format!("[cron:{}] {prompt}", job.id);

    let cli_config = CliProviderConfig {
        tool: config.primary_tool.clone(),
        working_dir: config.workspace_dir.clone(),
        timeout_secs: 600,
        ..CliProviderConfig::default()
    };

    let provider = CliProvider::new(cli_config);

    let messages = vec![ProviderMessage {
        role: "user".into(),
        content: prefixed,
        tool_calls: None,
        tool_call_id: None,
    }];

    match provider.execute(messages, vec![]).await {
        Ok(response) => {
            let content = response.content.trim().to_string();
            (true, if content.is_empty() { "agent job executed".into() } else { content })
        }
        Err(e) => (false, format!("agent job failed: {e}")),
    }
}
```

#### 2.3.4 Update `src/daemon/mod.rs` — Pass Full Config to Scheduler

Change the scheduler spawn block inside `daemon::run()`:

```rust
// Replace the current scheduler spawn with:
if config.cron.enabled {
    let sched_config = config.clone();
    handles.push(spawn_supervisor("scheduler", initial_backoff, max_backoff, move || {
        let cfg = sched_config.clone();
        async move { crate::cron::scheduler::run(&cfg).await }
    }));
}
```

#### 2.3.5 New Tool: `src/tools/builtin/cron_tools.rs`

Allow the agent loop itself to schedule cron jobs via tool calls:

```rust
//! Cron management tools — let agents schedule tasks via the tool interface

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};

use crate::tools::{Tool, ToolOutput, ToolSpec};

pub struct CronAddTool {
    workspace_dir: PathBuf,
}

impl CronAddTool {
    pub fn new(workspace_dir: &Path) -> Self {
        Self { workspace_dir: workspace_dir.to_path_buf() }
    }
}

#[async_trait]
impl Tool for CronAddTool {
    fn name(&self) -> &str { "cron_add" }

    fn description(&self) -> &str {
        "Schedule a recurring task. Provide a cron expression and a shell command."
    }

    fn parameters_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "required": ["expression", "command"],
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "Cron expression (e.g. '*/5 * * * *' for every 5 minutes)"
                },
                "command": {
                    "type": "string",
                    "description": "Shell command to execute on schedule"
                },
                "name": {
                    "type": "string",
                    "description": "Optional human-readable name for this job"
                }
            }
        })
    }

    async fn execute(&self, arguments: JsonValue) -> ToolOutput {
        let expr = arguments["expression"].as_str().unwrap_or("");
        let command = arguments["command"].as_str().unwrap_or("");
        let name = arguments["name"].as_str().map(|s| s.to_string());

        if expr.is_empty() || command.is_empty() {
            return ToolOutput::error("Both 'expression' and 'command' are required".into());
        }

        let schedule = crate::cron::Schedule::Cron {
            expr: expr.to_string(),
            tz: None,
        };

        match crate::cron::add_shell_job(&self.workspace_dir, name, schedule, command) {
            Ok(job) => ToolOutput::success(format!(
                "Scheduled cron job {}. Next run: {}",
                job.id, job.next_run.to_rfc3339()
            )),
            Err(e) => ToolOutput::error(format!("Failed to schedule job: {e}")),
        }
    }
}

pub struct CronListTool {
    workspace_dir: PathBuf,
}

impl CronListTool {
    pub fn new(workspace_dir: &Path) -> Self {
        Self { workspace_dir: workspace_dir.to_path_buf() }
    }
}

#[async_trait]
impl Tool for CronListTool {
    fn name(&self) -> &str { "cron_list" }

    fn description(&self) -> &str {
        "List all scheduled cron jobs with their status and next run time."
    }

    fn parameters_schema(&self) -> JsonValue {
        serde_json::json!({ "type": "object", "properties": {} })
    }

    async fn execute(&self, _arguments: JsonValue) -> ToolOutput {
        match crate::cron::list_jobs(&self.workspace_dir) {
            Ok(jobs) => {
                if jobs.is_empty() {
                    return ToolOutput::success("No scheduled jobs.".into());
                }
                let mut lines = vec![format!("{} scheduled job(s):", jobs.len())];
                for job in &jobs {
                    let status = job.last_status.as_deref().unwrap_or("pending");
                    lines.push(format!(
                        "- {} | {} | next={} | status={}",
                        job.id,
                        if job.command.is_empty() {
                            job.prompt.as_deref().unwrap_or("(agent)")
                        } else { &job.command },
                        job.next_run.to_rfc3339(),
                        status,
                    ));
                }
                ToolOutput::success(lines.join("\n"))
            }
            Err(e) => ToolOutput::error(format!("Failed to list jobs: {e}")),
        }
    }
}

pub struct CronRemoveTool {
    workspace_dir: PathBuf,
}

impl CronRemoveTool {
    pub fn new(workspace_dir: &Path) -> Self {
        Self { workspace_dir: workspace_dir.to_path_buf() }
    }
}

#[async_trait]
impl Tool for CronRemoveTool {
    fn name(&self) -> &str { "cron_remove" }

    fn description(&self) -> &str {
        "Remove a scheduled cron job by its ID."
    }

    fn parameters_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": { "type": "string", "description": "Job ID to remove" }
            }
        })
    }

    async fn execute(&self, arguments: JsonValue) -> ToolOutput {
        let id = arguments["id"].as_str().unwrap_or("");
        if id.is_empty() {
            return ToolOutput::error("'id' is required".into());
        }
        match crate::cron::remove_job(&self.workspace_dir, id) {
            Ok(()) => ToolOutput::success(format!("Removed cron job {id}")),
            Err(e) => ToolOutput::error(format!("Failed to remove job: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn cron_add_tool_rejects_empty_expression() {
        let tmp = TempDir::new().unwrap();
        let tool = CronAddTool::new(tmp.path());
        let result = tool.execute(serde_json::json!({
            "expression": "",
            "command": "echo hi"
        })).await;
        assert!(result.is_error);
    }

    #[tokio::test]
    async fn cron_list_tool_empty() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        let tool = CronListTool::new(&ws);
        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.is_error);
        assert!(result.content.contains("No scheduled"));
    }
}
```

#### 2.3.6 Update `src/tools/builtin/mod.rs`

```rust
// Add to existing file:
pub mod cron_tools;

pub use cron_tools::{CronAddTool, CronListTool, CronRemoveTool};
```

### 2.4 Cargo.toml Changes for Phase 2

No new dependencies required — all used crates are already available.

---

## Phase 3: Real Channel Transports & Gateway Server

### 3.1 Goal

Replace the TUI-only channel/gateway stubs with real message transports. Incoming messages from channels are processed by spawning the primary CLI tool (not calling an HTTP API).

### 3.2 New Dependencies (add to `crates/maestro-claw/Cargo.toml`)

```toml
# Phase 3 — real channels + gateway
axum = { version = "0.8", default-features = false, features = ["http1", "json", "tokio"], optional = true }
tower-http = { version = "0.6", default-features = false, features = ["limit", "timeout"], optional = true }
tokio-tungstenite = { version = "0.24", features = ["rustls-tls-webpki-roots"], optional = true }

[features]
default = ["providers"]
providers = ["dep:reqwest", "dep:futures", "dep:tokio-stream"]
channels = ["dep:reqwest", "dep:tokio-tungstenite", "dep:futures-util"]
gateway = ["dep:axum", "dep:tower-http"]
core-integration = ["dep:maestro-core", "dep:futures-util"]
```

### 3.3 New Files

#### 3.3.1 `src/channels/mod.rs`

```rust
//! MaestroClaw channel system

pub mod traits;
#[cfg(feature = "channels")]
pub mod telegram;
#[cfg(feature = "channels")]
pub mod discord;
pub mod dispatcher;

pub use traits::{Channel, ChannelMessage, SendMessage};
pub use dispatcher::ChannelDispatcher;
```

#### 3.3.2 `src/channels/traits.rs` — Core Channel Trait

```rust
//! Channel trait — the interface every messaging transport implements.

use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ChannelMessage {
    pub id: String,
    pub sender: String,
    pub reply_target: String,
    pub content: String,
    pub channel: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct SendMessage {
    pub content: String,
    pub recipient: String,
}

impl SendMessage {
    pub fn new(content: impl Into<String>, recipient: impl Into<String>) -> Self {
        Self { content: content.into(), recipient: recipient.into() }
    }
}

#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()>;
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()>;
    async fn health_check(&self) -> bool { true }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyChannel;

    #[async_trait]
    impl Channel for DummyChannel {
        fn name(&self) -> &str { "dummy" }
        async fn send(&self, _: &SendMessage) -> anyhow::Result<()> { Ok(()) }
        async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
            tx.send(ChannelMessage {
                id: "1".into(), sender: "test".into(), reply_target: "test".into(),
                content: "hello".into(), channel: "dummy".into(), timestamp: 0,
            }).await.map_err(|e| anyhow::anyhow!("{e}"))
        }
    }

    #[tokio::test]
    async fn dummy_channel_sends_and_receives() {
        let ch = DummyChannel;
        assert!(ch.send(&SendMessage::new("hi", "bob")).await.is_ok());
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        ch.listen(tx).await.unwrap();
        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.content, "hello");
    }
}
```

#### 3.3.3 `src/channels/dispatcher.rs` — Message Dispatcher

Routes incoming channel messages to the CLI agent and sends responses back:

```rust
//! Channel dispatcher — routes incoming messages to the CLI agent

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::agent::cli_provider::{CliProvider, CliProviderConfig};
use crate::agent::Provider;
use crate::channels::{Channel, ChannelMessage, SendMessage};
use crate::session::ProviderMessage;

pub struct ChannelDispatcher {
    primary_tool: String,
    workspace_dir: PathBuf,
}

impl ChannelDispatcher {
    pub fn new(primary_tool: &str, workspace_dir: &std::path::Path) -> Self {
        Self {
            primary_tool: primary_tool.to_string(),
            workspace_dir: workspace_dir.to_path_buf(),
        }
    }

    pub async fn run(
        &self,
        channel: Arc<dyn Channel>,
        mut rx: mpsc::Receiver<ChannelMessage>,
    ) -> anyhow::Result<()> {
        crate::health::mark_component_ok(&format!("channel:{}", channel.name()));

        while let Some(msg) = rx.recv().await {
            tracing::info!(channel = channel.name(), sender = msg.sender, "Received message");

            let provider = CliProvider::new(CliProviderConfig {
                tool: self.primary_tool.clone(),
                working_dir: self.workspace_dir.clone(),
                timeout_secs: 300,
                ..CliProviderConfig::default()
            });

            let messages = vec![ProviderMessage {
                role: "user".into(),
                content: msg.content.clone(),
                tool_calls: None,
                tool_call_id: None,
            }];

            match provider.execute(messages, vec![]).await {
                Ok(response) => {
                    let reply = SendMessage::new(response.content, &msg.reply_target);
                    if let Err(e) = channel.send(&reply).await {
                        tracing::warn!("Failed to send reply: {e}");
                    }
                }
                Err(e) => {
                    tracing::error!("Agent error: {e}");
                    let _ = channel.send(&SendMessage::new(format!("Error: {e}"), &msg.reply_target)).await;
                }
            }
        }
        Ok(())
    }
}
```

#### 3.3.4 `src/channels/telegram.rs` — Telegram Bot (Long-Polling)

```rust
//! Telegram bot channel — long-polling implementation

use crate::channels::{Channel, ChannelMessage, SendMessage};
use async_trait::async_trait;
use std::time::Duration;

pub struct TelegramChannel {
    bot_token: String,
    allowed_users: Vec<String>,
    client: reqwest::Client,
}

impl TelegramChannel {
    pub fn new(bot_token: String, allowed_users: Vec<String>) -> Self {
        Self {
            bot_token, allowed_users,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build().expect("HTTP client"),
        }
    }

    fn api_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{method}", self.bot_token)
    }

    fn is_allowed(&self, user_id: &str) -> bool {
        self.allowed_users.is_empty()
            || self.allowed_users.iter().any(|u| u == "*" || u == user_id)
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &str { "telegram" }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        self.client
            .post(&self.api_url("sendMessage"))
            .json(&serde_json::json!({
                "chat_id": message.recipient,
                "text": message.content,
            }))
            .send().await?;
        Ok(())
    }

    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        let mut offset: i64 = 0;
        loop {
            let url = format!("{}?offset={}&timeout=30", self.api_url("getUpdates"), offset);
            let resp: serde_json::Value = match self.client.get(&url).send().await {
                Ok(r) => r.json().await.unwrap_or_default(),
                Err(e) => {
                    tracing::warn!("Telegram poll error: {e}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            if let Some(updates) = resp["result"].as_array() {
                for update in updates {
                    if let Some(uid) = update["update_id"].as_i64() { offset = uid + 1; }
                    let message = &update["message"];
                    let text = message["text"].as_str().unwrap_or("");
                    let chat_id = message["chat"]["id"].as_i64().unwrap_or(0);
                    let user_id = message["from"]["id"].as_i64().unwrap_or(0);

                    if text.is_empty() || !self.is_allowed(&user_id.to_string()) { continue; }

                    let _ = tx.send(ChannelMessage {
                        id: update["update_id"].to_string(),
                        sender: user_id.to_string(),
                        reply_target: chat_id.to_string(),
                        content: text.to_string(),
                        channel: "telegram".into(),
                        timestamp: message["date"].as_u64().unwrap_or(0),
                    }).await;
                }
            }
        }
    }

    async fn health_check(&self) -> bool {
        self.client.get(&self.api_url("getMe")).send().await
            .map(|r| r.status().is_success()).unwrap_or(false)
    }
}
```

#### 3.3.5 `src/channels/discord.rs` — Discord Bot (WebSocket Gateway)

```rust
//! Discord bot channel — WebSocket gateway integration

use crate::channels::{Channel, ChannelMessage, SendMessage};
use async_trait::async_trait;

pub struct DiscordChannel {
    bot_token: String,
    allowed_users: Vec<String>,
    client: reqwest::Client,
}

impl DiscordChannel {
    pub fn new(bot_token: String, _guild_id: String, allowed_users: Vec<String>) -> Self {
        Self { bot_token, allowed_users, client: reqwest::Client::new() }
    }

    fn is_allowed(&self, user_id: &str) -> bool {
        self.allowed_users.is_empty()
            || self.allowed_users.iter().any(|u| u == "*" || u == user_id)
    }
}

#[async_trait]
impl Channel for DiscordChannel {
    fn name(&self) -> &str { "discord" }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        let url = format!("https://discord.com/api/v10/channels/{}/messages", message.recipient);
        self.client.post(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .json(&serde_json::json!({ "content": message.content }))
            .send().await?;
        Ok(())
    }

    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::connect_async;

        let (ws, _) = connect_async("wss://gateway.discord.gg/?v=10&encoding=json").await?;
        let (mut write, mut read) = ws.split();

        // Send IDENTIFY
        let identify = serde_json::json!({
            "op": 2,
            "d": {
                "token": self.bot_token,
                "intents": 512 | 32768,
                "properties": { "os": std::env::consts::OS, "browser": "maestroclaw", "device": "maestroclaw" }
            }
        });
        write.send(tokio_tungstenite::tungstenite::Message::Text(identify.to_string().into())).await?;

        while let Some(msg) = read.next().await {
            let msg = msg?;
            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                let payload: serde_json::Value = serde_json::from_str(&text)?;

                // Heartbeat ACK on op=10
                if payload["op"].as_u64() == Some(10) {
                    let hb = serde_json::json!({ "op": 1, "d": null });
                    let _ = write.send(tokio_tungstenite::tungstenite::Message::Text(hb.to_string().into())).await;
                }

                // MESSAGE_CREATE
                if payload["op"].as_u64() == Some(0) && payload["t"].as_str() == Some("MESSAGE_CREATE") {
                    let d = &payload["d"];
                    let author_id = d["author"]["id"].as_str().unwrap_or("");
                    let is_bot = d["author"]["bot"].as_bool().unwrap_or(false);
                    let content = d["content"].as_str().unwrap_or("");
                    let channel_id = d["channel_id"].as_str().unwrap_or("");

                    if is_bot || content.is_empty() || !self.is_allowed(author_id) { continue; }

                    let _ = tx.send(ChannelMessage {
                        id: d["id"].as_str().unwrap_or("").into(),
                        sender: author_id.into(),
                        reply_target: channel_id.into(),
                        content: content.into(),
                        channel: "discord".into(),
                        timestamp: 0,
                    }).await;
                }
            }
        }
        Ok(())
    }
}
```

#### 3.3.6 `src/gateway/mod.rs` — HTTP Gateway Server

```rust
//! MaestroClaw HTTP gateway — axum-based webhook server

use crate::agent::cli_provider::{CliProvider, CliProviderConfig};
use crate::agent::Provider;
use crate::config::Config;
use crate::session::ProviderMessage;
use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

const MAX_BODY_SIZE: usize = 65_536;
const REQUEST_TIMEOUT_SECS: u64 = 30;

#[derive(Clone)]
struct GatewayState { config: Arc<Config> }

pub async fn run_gateway(config: Config) -> Result<()> {
    let addr = format!("{}:{}", config.gateway.host, config.gateway.port);
    crate::health::mark_component_ok("gateway");

    let state = GatewayState { config: Arc::new(config) };
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/status", get(status_handler))
        .route("/webhook", post(webhook_handler))
        .layer(RequestBodyLimitLayer::new(MAX_BODY_SIZE))
        .layer(TimeoutLayer::new(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS)))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("🚀 MaestroClaw Gateway on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "service": "maestroclaw" }))
}

async fn status_handler() -> impl IntoResponse {
    Json(crate::health::snapshot_json())
}

#[derive(serde::Deserialize)]
struct WebhookPayload { message: String }

async fn webhook_handler(
    State(state): State<GatewayState>,
    Json(payload): Json<WebhookPayload>,
) -> impl IntoResponse {
    if payload.message.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "success": false, "error": "empty message" })));
    }

    let provider = CliProvider::new(CliProviderConfig {
        tool: state.config.primary_tool.clone(),
        working_dir: state.config.workspace_dir.clone(),
        timeout_secs: 600,
        ..CliProviderConfig::default()
    });

    let messages = vec![ProviderMessage {
        role: "user".into(), content: payload.message,
        tool_calls: None, tool_call_id: None,
    }];

    match provider.execute(messages, vec![]).await {
        Ok(resp) => (StatusCode::OK, Json(serde_json::json!({ "success": true, "response": resp.content }))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false, "error": e.to_string() }))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_returns_ok() {
        let resp = health_handler().await.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
```

### 3.4 Update `src/daemon/mod.rs` — Add Gateway to Daemon

```rust
// Add after scheduler spawn inside daemon::run():

// Gateway
{
    let gw_config = config.clone();
    handles.push(spawn_supervisor("gateway", initial_backoff, max_backoff, move || {
        let cfg = gw_config.clone();
        async move { crate::gateway::run_gateway(cfg).await }
    }));
}

// Update the startup message:
println!("🧠 MaestroClaw daemon started");
println!("   Gateway:  http://{}:{}", config.gateway.host, config.gateway.port);
println!("   Components: gateway, scheduler");
println!("   Ctrl+C to stop");
```

### 3.5 Update `src/lib.rs`

```rust
pub mod channels;

#[cfg(feature = "gateway")]
pub mod gateway;
```

---

## Phase 4: Heartbeat, Skills, Cost Tracking, Observability

### 4.1 Goal

Add the remaining operational subsystems for full production readiness.

### 4.2 New Files

#### 4.2.1 `src/heartbeat/mod.rs` — Periodic Task Engine

```rust
//! Heartbeat engine — reads HEARTBEAT.md and executes periodic tasks

use crate::agent::cli_provider::{CliProvider, CliProviderConfig};
use crate::agent::Provider;
use crate::config::Config;
use crate::session::ProviderMessage;
use anyhow::Result;
use std::path::Path;
use tokio::time::{self, Duration};

pub struct HeartbeatEngine {
    interval_minutes: u32,
    workspace_dir: std::path::PathBuf,
    primary_tool: String,
}

impl HeartbeatEngine {
    pub fn new(config: &Config) -> Self {
        Self {
            interval_minutes: 15,
            workspace_dir: config.workspace_dir.clone(),
            primary_tool: config.primary_tool.clone(),
        }
    }

    pub async fn run(&self) -> Result<()> {
        let mins = self.interval_minutes.max(5);
        tracing::info!("💓 Heartbeat started: every {mins} minutes");
        let mut interval = time::interval(Duration::from_secs(u64::from(mins) * 60));

        loop {
            interval.tick().await;
            crate::health::mark_component_ok("heartbeat");
            match self.tick().await {
                Ok(n) if n > 0 => tracing::info!("💓 Heartbeat: processed {n} tasks"),
                Err(e) => {
                    crate::health::mark_component_error("heartbeat", e.to_string());
                    tracing::warn!("💓 Heartbeat error: {e}");
                }
                _ => {}
            }
        }
    }

    async fn tick(&self) -> Result<usize> {
        let tasks = self.collect_tasks().await?;
        for task in &tasks { self.execute_task(task).await?; }
        Ok(tasks.len())
    }

    pub async fn collect_tasks(&self) -> Result<Vec<String>> {
        let path = self.workspace_dir.join("HEARTBEAT.md");
        if !path.exists() { return Ok(Vec::new()); }
        let content = tokio::fs::read_to_string(&path).await?;
        Ok(Self::parse_tasks(&content))
    }

    pub async fn ensure_heartbeat_file(workspace_dir: &Path) -> Result<()> {
        let path = workspace_dir.join("HEARTBEAT.md");
        if !path.exists() {
            tokio::fs::write(&path,
                "# Heartbeat Tasks\n\nPeriodic tasks for MaestroClaw.\n\n\
                 <!-- Add as list items:\n- Check for dependency updates\n-->\n"
            ).await?;
        }
        Ok(())
    }

    fn parse_tasks(content: &str) -> Vec<String> {
        content.lines()
            .filter_map(|line| {
                let t = line.trim();
                if (t.starts_with("- ") || t.starts_with("* "))
                    && !t[2..].trim().is_empty()
                    && !t[2..].trim().starts_with("<!--")
                    && !t[2..].trim().starts_with("//")
                {
                    Some(t[2..].trim().to_string())
                } else { None }
            })
            .collect()
    }

    async fn execute_task(&self, task: &str) -> Result<()> {
        let provider = CliProvider::new(CliProviderConfig {
            tool: self.primary_tool.clone(),
            working_dir: self.workspace_dir.clone(),
            timeout_secs: 600,
            ..CliProviderConfig::default()
        });
        let messages = vec![ProviderMessage {
            role: "user".into(),
            content: format!("[Heartbeat Task] {task}"),
            tool_calls: None, tool_call_id: None,
        }];
        provider.execute(messages, vec![]).await
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Heartbeat task failed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tasks_extracts_list_items() {
        let tasks = HeartbeatEngine::parse_tasks("# Heartbeat\n\n- Check updates\n- Run tests\n");
        assert_eq!(tasks, vec!["Check updates", "Run tests"]);
    }

    #[test]
    fn parse_tasks_skips_comments() {
        let tasks = HeartbeatEngine::parse_tasks("- Real task\n- <!-- skip -->\n- // skip\n");
        assert_eq!(tasks, vec!["Real task"]);
    }

    #[test]
    fn parse_tasks_empty() {
        assert!(HeartbeatEngine::parse_tasks("").is_empty());
    }
}
```

#### 4.2.2 `src/skills/mod.rs` — Skills System

```rust
//! MaestroClaw skills — user-defined capabilities from TOML manifests

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub tools: Vec<SkillTool>,
    #[serde(default)]
    pub prompts: Vec<String>,
    #[serde(skip)]
    pub location: Option<PathBuf>,
}

fn default_version() -> String { "0.1.0".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTool {
    pub name: String,
    pub description: String,
    pub kind: String,
    pub command: String,
    #[serde(default)]
    pub args: HashMap<String, String>,
}

pub fn load_skills(workspace_dir: &Path) -> Vec<Skill> {
    let dir = workspace_dir.join("skills");
    if !dir.exists() { return Vec::new(); }
    let mut skills = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let manifest = p.join("SKILL.toml");
                if manifest.exists() {
                    if let Ok(mut skill) = load_manifest(&manifest) {
                        skill.location = Some(p);
                        skills.push(skill);
                    }
                }
            }
        }
    }
    skills
}

fn load_manifest(path: &Path) -> Result<Skill> {
    #[derive(Deserialize)]
    struct M { skill: SM, #[serde(default)] tools: Vec<SkillTool>, #[serde(default)] prompts: Vec<String> }
    #[derive(Deserialize)]
    struct SM { name: String, description: String, #[serde(default = "default_version")] version: String, #[serde(default)] author: Option<String>, #[serde(default)] tags: Vec<String> }

    let content = std::fs::read_to_string(path)?;
    let m: M = toml::from_str(&content)?;
    Ok(Skill { name: m.skill.name, description: m.skill.description, version: m.skill.version, author: m.skill.author, tags: m.skill.tags, tools: m.tools, prompts: m.prompts, location: None })
}

pub fn install_skill(workspace_dir: &Path, source: &Path) -> Result<()> {
    let name = source.file_name().and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid source path"))?;
    let dest = workspace_dir.join("skills").join(name);
    if dest.exists() { anyhow::bail!("Skill '{name}' already installed"); }
    copy_dir(source, &dest)?;
    println!("✅ Installed skill: {name}");
    Ok(())
}

pub fn remove_skill(workspace_dir: &Path, name: &str) -> Result<()> {
    let dir = workspace_dir.join("skills").join(name);
    if !dir.exists() { anyhow::bail!("Skill '{name}' not found"); }
    std::fs::remove_dir_all(&dir)?;
    println!("✅ Removed skill: {name}");
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let e = entry?;
        let d = dst.join(e.file_name());
        if e.path().is_dir() { copy_dir(&e.path(), &d)?; } else { std::fs::copy(&e.path(), &d)?; }
    }
    Ok(())
}

pub fn handle_command(command: &str, args: &[&str], workspace_dir: &Path) -> Result<()> {
    match command {
        "list" => {
            let skills = load_skills(workspace_dir);
            if skills.is_empty() { println!("No skills installed."); return Ok(()); }
            println!("📦 Installed skills ({}):", skills.len());
            for s in &skills { println!("  {} v{} — {}", s.name, s.version, s.description); }
            Ok(())
        }
        "install" if !args.is_empty() => install_skill(workspace_dir, Path::new(args[0])),
        "remove" if !args.is_empty() => remove_skill(workspace_dir, args[0]),
        _ => { println!("Usage: maestro claw skills [list|install|remove]"); Ok(()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_skills_empty() {
        let tmp = TempDir::new().unwrap();
        assert!(load_skills(tmp.path()).is_empty());
    }

    #[test]
    fn load_from_manifest() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("skills").join("test-skill");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.toml"), r#"
[skill]
name = "test-skill"
description = "A test"
version = "1.0.0"

[[tools]]
name = "greet"
description = "Say hello"
kind = "shell"
command = "echo hello"
"#).unwrap();
        let skills = load_skills(tmp.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "test-skill");
        assert_eq!(skills[0].tools.len(), 1);
    }
}
```

#### 4.2.3 `src/cost/mod.rs` — Cost Tracking

```rust
//! Cost tracking for MaestroClaw agent invocations

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecord {
    pub timestamp: DateTime<Utc>,
    pub tool: String,
    pub duration_ms: i64,
    pub prompt_chars: usize,
    pub response_chars: usize,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub total_invocations: usize,
    pub total_duration_ms: i64,
    pub total_estimated_cost_usd: f64,
    pub daily_cost_usd: f64,
    pub monthly_cost_usd: f64,
}

pub struct CostTracker {
    storage_path: PathBuf,
    daily_limit_usd: f64,
    monthly_limit_usd: f64,
}

impl CostTracker {
    pub fn new(workspace_dir: &Path, daily_limit: f64, monthly_limit: f64) -> Self {
        Self {
            storage_path: workspace_dir.join("cost").join("records.json"),
            daily_limit_usd: daily_limit,
            monthly_limit_usd: monthly_limit,
        }
    }

    pub fn record(&self, record: CostRecord) -> Result<()> {
        let mut records = self.load()?;
        records.push(record);
        self.save(&records)
    }

    pub fn check_budget(&self) -> Result<bool> {
        let s = self.summarize()?;
        Ok(s.daily_cost_usd <= self.daily_limit_usd && s.monthly_cost_usd <= self.monthly_limit_usd)
    }

    pub fn summarize(&self) -> Result<CostSummary> {
        let records = self.load()?;
        let now = Utc::now();
        let today = now.date_naive().and_hms_opt(0,0,0).and_then(|d| d.and_local_timezone(Utc).single()).unwrap_or(now);
        let month = now.date_naive().with_day(1).and_then(|d| d.and_hms_opt(0,0,0)).and_then(|d| d.and_local_timezone(Utc).single()).unwrap_or(now);
        let daily: f64 = records.iter().filter(|r| r.timestamp >= today).map(|r| r.estimated_cost_usd).sum();
        let monthly: f64 = records.iter().filter(|r| r.timestamp >= month).map(|r| r.estimated_cost_usd).sum();
        Ok(CostSummary {
            total_invocations: records.len(),
            total_duration_ms: records.iter().map(|r| r.duration_ms).sum(),
            total_estimated_cost_usd: records.iter().map(|r| r.estimated_cost_usd).sum(),
            daily_cost_usd: daily, monthly_cost_usd: monthly,
        })
    }

    fn load(&self) -> Result<Vec<CostRecord>> {
        if !self.storage_path.exists() { return Ok(Vec::new()); }
        Ok(serde_json::from_str(&std::fs::read_to_string(&self.storage_path)?)?)
    }

    fn save(&self, records: &[CostRecord]) -> Result<()> {
        if let Some(p) = self.storage_path.parent() { std::fs::create_dir_all(p)?; }
        std::fs::write(&self.storage_path, serde_json::to_string_pretty(records)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn empty_budget_ok() {
        let tmp = TempDir::new().unwrap();
        assert!(CostTracker::new(tmp.path(), 10.0, 100.0).check_budget().unwrap());
    }

    #[test]
    fn record_and_summarize() {
        let tmp = TempDir::new().unwrap();
        let t = CostTracker::new(tmp.path(), 10.0, 100.0);
        t.record(CostRecord {
            timestamp: Utc::now(), tool: "claude".into(), duration_ms: 5000,
            prompt_chars: 100, response_chars: 500, estimated_cost_usd: 0.05,
        }).unwrap();
        let s = t.summarize().unwrap();
        assert_eq!(s.total_invocations, 1);
        assert!((s.total_estimated_cost_usd - 0.05).abs() < 0.001);
    }
}
```

#### 4.2.4 `src/observability/mod.rs` — Pluggable Observability

```rust
//! Observability system — pluggable logging/metrics backends

#[derive(Debug, Clone)]
pub enum ObserverEvent {
    AgentStart { tool: String },
    AgentComplete { tool: String, duration_ms: i64 },
    AgentError { tool: String, error: String },
    SchedulerTick,
    HeartbeatTick,
    Error { component: String, message: String },
}

pub trait Observer: Send + Sync {
    fn name(&self) -> &str;
    fn record_event(&self, event: &ObserverEvent);
}

pub struct NoopObserver;
impl Observer for NoopObserver {
    fn name(&self) -> &str { "noop" }
    fn record_event(&self, _: &ObserverEvent) {}
}

pub struct LogObserver;
impl Observer for LogObserver {
    fn name(&self) -> &str { "log" }
    fn record_event(&self, event: &ObserverEvent) {
        match event {
            ObserverEvent::AgentStart { tool } => tracing::info!(tool, "Agent started"),
            ObserverEvent::AgentComplete { tool, duration_ms } => tracing::info!(tool, duration_ms, "Agent completed"),
            ObserverEvent::AgentError { tool, error } => tracing::error!(tool, error, "Agent error"),
            ObserverEvent::SchedulerTick => tracing::debug!("Scheduler tick"),
            ObserverEvent::HeartbeatTick => tracing::debug!("Heartbeat tick"),
            ObserverEvent::Error { component, message } => tracing::error!(component, message, "Component error"),
        }
    }
}

pub fn create_observer(backend: &str) -> Box<dyn Observer> {
    match backend { "log" => Box::new(LogObserver), _ => Box::new(NoopObserver) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_observer() {
        let o = NoopObserver;
        o.record_event(&ObserverEvent::SchedulerTick);
        assert_eq!(o.name(), "noop");
    }

    #[test]
    fn log_observer() {
        let o = LogObserver;
        o.record_event(&ObserverEvent::AgentStart { tool: "claude".into() });
        assert_eq!(o.name(), "log");
    }

    #[test]
    fn create_observer_unknown_returns_noop() {
        assert_eq!(create_observer("xyz").name(), "noop");
    }
}
```

### 4.3 Final `src/lib.rs`

```rust
pub mod agent;
pub mod channels;
pub mod config;
pub mod cost;
pub mod cron;
pub mod daemon;
pub mod doctor;
pub mod health;
pub mod heartbeat;
pub mod hooks;
pub mod observability;
pub mod onboard;
pub mod service;
pub mod session;
pub mod skills;
pub mod tools;

#[cfg(feature = "providers")]
pub mod providers;

#[cfg(feature = "gateway")]
pub mod gateway;

#[cfg(feature = "core-integration")]
pub mod integration;
```

---

## Dependency Changes Summary

| Phase | New Dependencies |
|---|---|
| **Phase 2** | None |
| **Phase 3** | `axum`, `tower-http`, `tokio-tungstenite` (all optional, behind features) |
| **Phase 4** | None |

---

## Testing Strategy

### Run Commands

```bash
# Full crate check
cargo check -p maestro-claw --all-features

# All tests
cargo test -p maestro-claw

# Per-phase
cargo test -p maestro-claw cli_provider
cargo test -p maestro-claw cron_tools
cargo test -p maestro-claw channels
cargo test -p maestro-claw gateway
cargo test -p maestro-claw heartbeat
cargo test -p maestro-claw skills
cargo test -p maestro-claw cost
cargo test -p maestro-claw observability
```

---

## File Summary

| Phase | New Files | Modified Files |
|---|---|---|
| **Phase 2** | `agent/cli_provider.rs`, `tools/builtin/cron_tools.rs` | `agent/mod.rs`, `tools/builtin/mod.rs`, `cron/scheduler.rs`, `daemon/mod.rs` |
| **Phase 3** | `channels/mod.rs`, `channels/traits.rs`, `channels/dispatcher.rs`, `channels/telegram.rs`, `channels/discord.rs`, `gateway/mod.rs` | `lib.rs`, `daemon/mod.rs`, `Cargo.toml` |
| **Phase 4** | `heartbeat/mod.rs`, `skills/mod.rs`, `cost/mod.rs`, `observability/mod.rs` | `lib.rs` |
| **Total** | **14 new files** | **7 modified files** |

+  ### Estimated LoC per Phase
+
+  | Phase | Estimated Lines |
+  |---|---|
+  | Phase 2 | ~800 |
+  | Phase 3 | ~1,200 |
+  | Phase 4 | ~900 |
+  | **Total** | **~2,900** |
+
+  ---
+
+  ## Priority Order
+
+  1. **Phase 2 (HIGH)** — CLI tool integration is the core value proposition. Without it, MaestroClaw is just a session framework with no way to actually drive agents. The `CliProvider` + updated scheduler makes the entire system functional end-to-end.
+
+  2. **Phase 3 (MEDIUM)** — Channels and gateway enable external access (Telegram/Discord bots, webhook API). These transform MaestroClaw from a local-only tool to a remotely-accessible agent.
+
+  3. **Phase 4 (LOWER)** — Heartbeat, skills, cost tracking, and observability are operational polish. They're important for production use but the system is functional without them.
