//! CLI tool provider for locally-installed coding agents.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value as JsonValue;
use tokio::process::Command;
use tokio::time::timeout;
use uuid::Uuid;

use crate::agent::{AgentError, Provider, ProviderResponse};
use crate::cost::{CostEstimator, CostRecord, CostTracker, InvocationType};
use crate::observability::{create_observer, ObserverEvent, TelemetryCorrelation};
use crate::session::{ProviderMessage, ToolCall};
use crate::tools::ToolSpec;

/// Known CLI tool identifiers supported by MaestroClaw.
pub const KNOWN_TOOLS: &[&str] = &["claude", "codex", "gemini", "qwen", "iflow", "amp", "droid"];

/// Runtime configuration for the CLI provider.
#[derive(Debug, Clone)]
pub struct CliProviderConfig {
    /// Tool binary name or absolute path.
    pub tool: String,
    /// Working directory used for the subprocess invocation.
    pub working_dir: PathBuf,
    /// Maximum execution time for a single provider turn.
    pub timeout_secs: u64,
    /// Extra tool-specific arguments appended after the default ones.
    pub extra_args: Vec<String>,
    /// Whether to enable single-shot print mode where supported.
    pub print_mode: bool,
    /// Observability backend used for provider execution events.
    pub observability_backend: String,
    /// Whether to persist cost records for provider executions.
    pub record_costs: bool,
    /// Daily spend ceiling used by the cost tracker.
    pub daily_cost_limit_usd: f64,
    /// Monthly spend ceiling used by the cost tracker.
    pub monthly_cost_limit_usd: f64,
}

impl Default for CliProviderConfig {
    fn default() -> Self {
        Self {
            tool: "claude".into(),
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            timeout_secs: 300,
            extra_args: Vec::new(),
            print_mode: true,
            observability_backend: "log".into(),
            record_costs: true,
            daily_cost_limit_usd: f64::MAX,
            monthly_cost_limit_usd: f64::MAX,
        }
    }
}

/// Provider implementation backed by locally-installed coding agent CLIs.
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
            .map(|status| status.success())
            .unwrap_or(false)
    }

    pub fn detect_best_tool(working_dir: &Path) -> Option<Self> {
        KNOWN_TOOLS.iter().find_map(|tool| {
            let provider = Self::for_tool(tool, working_dir);
            provider.is_available().then_some(provider)
        })
    }

    fn build_args(&self, prompt: &str) -> Vec<String> {
        let mut args = match self.config.tool.as_str() {
            "claude" => {
                let mut args = Vec::new();
                if self.config.print_mode {
                    args.push("--print".into());
                }
                args.push("--message".into());
                args.push(prompt.to_string());
                args
            }
            "codex" => vec![
                "--quiet".into(),
                "--approval-mode".into(),
                "full-auto".into(),
                prompt.to_string(),
            ],
            "gemini" => vec!["--prompt".into(), prompt.to_string()],
            "qwen" => vec!["chat".into(), "--message".into(), prompt.to_string()],
            "iflow" => vec!["-p".into(), prompt.to_string()],
            "amp" => vec!["--message".into(), prompt.to_string()],
            "droid" => vec!["--prompt".into(), prompt.to_string()],
            _ => vec![prompt.to_string()],
        };

        args.extend(self.config.extra_args.clone());
        args
    }

    fn messages_to_prompt(messages: &[ProviderMessage], tools: &[ToolSpec]) -> String {
        let mut parts = Vec::new();

        for message in messages {
            match message.role.as_str() {
                "system" => parts.push(format!("[System]\n{}", message.content)),
                "user" => parts.push(message.content.clone()),
                "assistant" => parts.push(format!(
                    "[Previous Assistant Response]\n{}",
                    message.content
                )),
                "tool" => {
                    let tool_id = message.tool_call_id.as_deref().unwrap_or("unknown");
                    parts.push(format!("[Tool Result: {tool_id}]\n{}", message.content));
                }
                _ => parts.push(message.content.clone()),
            }
        }

        if !tools.is_empty() {
            let tool_descriptions = tools
                .iter()
                .map(|tool| {
                    format!(
                        "- {}: {}\n  schema: {}",
                        tool.name, tool.description, tool.parameters
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            parts.push(format!(
                "[Available Tools]\n{tool_descriptions}\n\n\
                 If you need a tool, respond with JSON only using this schema:\n\
                 {{\"content\":\"optional\",\"tool_calls\":[{{\"id\":\"call_1\",\"name\":\"tool_name\",\"arguments\":{{}}}}]}}\n\
                 If no tool is needed, respond with plain text."
            ));
        }

        parts.join("\n\n")
    }

    fn extract_json_payload(raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.starts_with("```") {
            let mut lines = trimmed.lines();
            let _opening = lines.next()?;
            let mut inner = lines.collect::<Vec<_>>();
            if matches!(inner.last(), Some(line) if line.trim() == "```") {
                inner.pop();
            }
            let payload = inner.join("\n").trim().to_string();
            if payload.is_empty() {
                None
            } else {
                Some(payload)
            }
        } else {
            Some(trimmed.to_string())
        }
    }

    fn normalize_arguments(arguments: JsonValue) -> JsonValue {
        match arguments {
            JsonValue::String(raw) => {
                serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({ "input": raw }))
            }
            other => other,
        }
    }

    fn parse_tool_call(value: &JsonValue) -> Option<ToolCall> {
        let id = value
            .get("id")
            .and_then(|id| id.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("cli-tool-{}", Uuid::new_v4()));

        if let Some(name) = value.get("name").and_then(|name| name.as_str()) {
            let arguments = value
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            return Some(ToolCall::new(
                id,
                name.to_string(),
                Self::normalize_arguments(arguments),
            ));
        }

        let function = value.get("function")?;
        let name = function.get("name").and_then(|name| name.as_str())?;
        let arguments = function
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        Some(ToolCall::new(
            id,
            name.to_string(),
            Self::normalize_arguments(arguments),
        ))
    }

    fn parse_response(raw: &str) -> ProviderResponse {
        let Some(payload) = Self::extract_json_payload(raw) else {
            return ProviderResponse::text(raw.to_string());
        };

        let Ok(value) = serde_json::from_str::<JsonValue>(&payload) else {
            return ProviderResponse::text(raw.to_string());
        };

        let Some(tool_calls_value) = value.get("tool_calls").and_then(|calls| calls.as_array())
        else {
            return ProviderResponse::text(raw.to_string());
        };

        let tool_calls = tool_calls_value
            .iter()
            .filter_map(Self::parse_tool_call)
            .collect::<Vec<_>>();

        if tool_calls.is_empty() {
            ProviderResponse::text(raw.to_string())
        } else {
            let content = value
                .get("content")
                .and_then(|content| content.as_str())
                .unwrap_or_default()
                .to_string();
            ProviderResponse::with_tools(content, tool_calls)
        }
    }

    fn correlation_for_messages(
        &self,
        messages: &[ProviderMessage],
        invocation_type: InvocationType,
    ) -> TelemetryCorrelation {
        let turn_index = messages
            .iter()
            .filter(|message| message.role != "system")
            .count()
            .checked_sub(1);
        let tool_call_id = messages
            .iter()
            .rev()
            .find_map(|message| message.tool_call_id.clone());
        let principal = messages
            .iter()
            .rev()
            .find(|message| message.role == "user")
            .map(|_| "user".to_string());
        let surface = match invocation_type {
            InvocationType::CronJob => "cron",
            InvocationType::HeartbeatTask => "heartbeat",
            InvocationType::ChannelMessage => "channel",
            InvocationType::Daemon => "daemon",
            InvocationType::Runtime => "runtime",
            InvocationType::Direct => "agent",
        };

        TelemetryCorrelation {
            session_id: None,
            thread_id: tool_call_id.clone(),
            turn_index,
            tool_call_id,
            principal,
            sender: None,
            surface: Some(surface.to_string()),
            component: Some(format!("agent:{}", self.config.tool)),
        }
    }

    async fn run_tool(
        &self,
        prompt: &str,
        invocation_type: InvocationType,
        correlation: TelemetryCorrelation,
    ) -> Result<String, AgentError> {
        let observer = create_observer(
            &self.config.observability_backend,
            Some(&self.config.working_dir),
        );
        let started_at = Instant::now();
        let prompt_chars = prompt.chars().count();
        observer.record_correlated_event(
            &ObserverEvent::AgentStart {
                tool: self.config.tool.clone(),
            },
            correlation.clone(),
        );

        let args = self.build_args(prompt);

        let mut command = Command::new(&self.config.tool);
        command
            .args(&args)
            .current_dir(&self.config.working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null())
            .env("CI", "true")
            .env("NONINTERACTIVE", "1");

        let output = timeout(
            Duration::from_secs(self.config.timeout_secs),
            command.output(),
        )
        .await
        .map_err(|_| {
            let error = AgentError::TimeoutExceeded(self.config.timeout_secs);
            observer.record_correlated_event(
                &ObserverEvent::AgentError {
                    tool: self.config.tool.clone(),
                    error: error.to_string(),
                },
                correlation.clone(),
            );
            error
        })?
        .map_err(|error| {
            let error = AgentError::ProviderError(format!(
                "Failed to spawn '{}': {error}",
                self.config.tool
            ));
            observer.record_correlated_event(
                &ObserverEvent::AgentError {
                    tool: self.config.tool.clone(),
                    error: error.to_string(),
                },
                correlation.clone(),
            );
            error
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        let duration_ms = started_at.elapsed().as_millis() as i64;

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            let message = if stderr.is_empty() {
                format!("'{}' exited with code {exit_code}", self.config.tool)
            } else {
                format!(
                    "'{}' exited with code {exit_code}: {stderr}",
                    self.config.tool
                )
            };
            observer.record_correlated_event(
                &ObserverEvent::AgentError {
                    tool: self.config.tool.clone(),
                    error: message.clone(),
                },
                correlation.clone(),
            );

            if self.config.record_costs {
                let tracker = CostTracker::new(
                    &self.config.working_dir,
                    self.config.daily_cost_limit_usd,
                    self.config.monthly_cost_limit_usd,
                );
                let response_chars = stderr.chars().count();
                let estimated_cost = CostEstimator::estimate_cost(
                    &self.config.tool,
                    prompt_chars,
                    response_chars,
                    None,
                );
                let _ = tracker.record(CostRecord {
                    timestamp: Utc::now(),
                    tool: self.config.tool.clone(),
                    provider: "cli".into(),
                    model: None,
                    duration_ms,
                    prompt_chars,
                    response_chars,
                    estimated_cost_usd: estimated_cost,
                    success: false,
                    error_message: Some(message.clone()),
                    invocation_type,
                    workspace_dir: Some(self.config.working_dir.to_string_lossy().to_string()),
                    session_id: correlation.session_id.clone(),
                    component: correlation.component.clone(),
                    correlation: Some(correlation.clone()),
                });
            }

            return Err(AgentError::ProviderError(message));
        }

        let response = if stdout.is_empty() { stderr } else { stdout };

        if self.config.record_costs {
            let tracker = CostTracker::new(
                &self.config.working_dir,
                self.config.daily_cost_limit_usd,
                self.config.monthly_cost_limit_usd,
            );
            let response_chars = response.chars().count();
            let estimated_cost =
                CostEstimator::estimate_cost(&self.config.tool, prompt_chars, response_chars, None);
            let _ = tracker.record(CostRecord {
                timestamp: Utc::now(),
                tool: self.config.tool.clone(),
                provider: "cli".into(),
                model: None,
                duration_ms,
                prompt_chars,
                response_chars,
                estimated_cost_usd: estimated_cost,
                success: true,
                error_message: None,
                invocation_type,
                workspace_dir: Some(self.config.working_dir.to_string_lossy().to_string()),
                session_id: correlation.session_id.clone(),
                component: correlation.component.clone(),
                correlation: Some(correlation.clone()),
            });
        }

        observer.record_correlated_event(
            &ObserverEvent::AgentComplete {
                tool: self.config.tool.clone(),
                duration_ms,
            },
            correlation,
        );

        Ok(response)
    }
}

#[async_trait]
impl Provider for CliProvider {
    async fn execute(
        &self,
        messages: Vec<ProviderMessage>,
        tools: Vec<ToolSpec>,
    ) -> Result<ProviderResponse, AgentError> {
        let prompt = Self::messages_to_prompt(&messages, &tools);
        if prompt.trim().is_empty() {
            return Err(AgentError::ConfigError("Empty prompt".into()));
        }

        let correlation = self.correlation_for_messages(&messages, InvocationType::Direct);
        let content = self
            .run_tool(&prompt, InvocationType::Direct, correlation)
            .await?;
        Ok(Self::parse_response(&content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let config = CliProviderConfig::default();
        assert_eq!(config.tool, "claude");
        assert_eq!(config.timeout_secs, 300);
        assert!(config.print_mode);
        assert_eq!(config.observability_backend, "log");
        assert!(config.record_costs);
    }

    #[test]
    fn build_args_for_claude_uses_message_mode() {
        let provider = CliProvider::new(CliProviderConfig {
            tool: "claude".into(),
            print_mode: true,
            ..CliProviderConfig::default()
        });

        assert_eq!(
            provider.build_args("hello"),
            vec![
                "--print".to_string(),
                "--message".to_string(),
                "hello".to_string()
            ]
        );
    }

    #[test]
    fn build_args_for_codex_uses_noninteractive_mode() {
        let provider = CliProvider::new(CliProviderConfig {
            tool: "codex".into(),
            ..CliProviderConfig::default()
        });

        assert_eq!(
            provider.build_args("hello"),
            vec![
                "--quiet".to_string(),
                "--approval-mode".to_string(),
                "full-auto".to_string(),
                "hello".to_string()
            ]
        );
    }

    #[test]
    fn build_args_for_qwen_uses_chat_message_form() {
        let provider = CliProvider::new(CliProviderConfig {
            tool: "qwen".into(),
            ..CliProviderConfig::default()
        });

        assert_eq!(
            provider.build_args("hello"),
            vec![
                "chat".to_string(),
                "--message".to_string(),
                "hello".to_string()
            ]
        );
    }

    #[test]
    fn build_args_appends_extra_args() {
        let provider = CliProvider::new(CliProviderConfig {
            tool: "amp".into(),
            extra_args: vec!["--foo".into(), "bar".into()],
            ..CliProviderConfig::default()
        });

        assert_eq!(
            provider.build_args("hello"),
            vec![
                "--message".to_string(),
                "hello".to_string(),
                "--foo".to_string(),
                "bar".to_string()
            ]
        );
    }

    #[test]
    fn messages_to_prompt_preserves_role_context() {
        let prompt = CliProvider::messages_to_prompt(
            &[
                ProviderMessage {
                    role: "system".into(),
                    content: "You are helpful".into(),
                    tool_calls: None,
                    tool_call_id: None,
                },
                ProviderMessage {
                    role: "user".into(),
                    content: "Solve it".into(),
                    tool_calls: None,
                    tool_call_id: None,
                },
                ProviderMessage {
                    role: "tool".into(),
                    content: "done".into(),
                    tool_calls: None,
                    tool_call_id: Some("call-1".into()),
                },
            ],
            &[],
        );

        assert!(prompt.contains("[System]"));
        assert!(prompt.contains("Solve it"));
        assert!(prompt.contains("[Tool Result: call-1]"));
    }

    #[test]
    fn parse_response_detects_tool_calls() {
        let response = CliProvider::parse_response(
            r#"{"content":"Using a tool","tool_calls":[{"name":"cron_add","arguments":{"expression":"*/5 * * * *","command":"echo hi"}}]}"#,
        );

        assert_eq!(response.content, "Using a tool");
        assert!(!response.is_finished);
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].name, "cron_add");
    }

    #[test]
    fn messages_to_prompt_includes_tool_contract() {
        let prompt = CliProvider::messages_to_prompt(
            &[ProviderMessage {
                role: "user".into(),
                content: "Schedule a task".into(),
                tool_calls: None,
                tool_call_id: None,
            }],
            &[ToolSpec {
                name: "cron_add".into(),
                description: "Schedule a job".into(),
                parameters: serde_json::json!({"type":"object"}),
            }],
        );

        assert!(prompt.contains("[Available Tools]"));
        assert!(prompt.contains("cron_add"));
        assert!(prompt.contains("\"tool_calls\""));
    }
}
