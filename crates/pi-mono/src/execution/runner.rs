//! # Subagent runner for pi-mono CLI execution
//!
//! This module provides the SubagentRunner for executing pi-mono CLI commands.
//!
//! ## Example
//!
//! ```rust,ignore
//! use maestro_pi_mono::execution::runner::{SubagentRunner, RunnerConfig};
//! use maestro_pi_mono::agents::mapping::PiAgentType;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create a runner with default configuration
//! let runner = SubagentRunner::new();
//!
//! // Run a subagent task
//! let result = runner.run(
//!     PiAgentType::Scout,
//!     "Analyze the codebase structure",
//!     None
//! ).await?;
//!
//! println!("Result: {}", result.output);
//! # Ok(())
//! # }
//! ```

use crate::{
    detection::PiDetection,
    execution::{SubagentResult, StreamEvent},
    error::{Result, Error},
    agents::mapping::{PiAgentType},
};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::process::Stdio;

/// Configuration for running subagents
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    pub pi_path: PathBuf,
    pub timeout: Duration,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub max_retries: usize,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            pi_path: PathBuf::from("pi"),
            timeout: Duration::from_secs(300),
            provider: None,
            model: None,
            max_retries: 3,
        }
    }
}

/// Subagent runner for executing pi-mono commands
pub struct SubagentRunner {
    config: RunnerConfig,
}

impl SubagentRunner {
    /// Create a new runner with default config
    pub fn new() -> Self {
        Self {
            config: RunnerConfig::default(),
        }
    }

    /// Create a new runner with custom config
    pub fn with_config(config: RunnerConfig) -> Self {
        Self { config }
    }

    /// Create a new runner from pi detection
    pub fn from_detection(detection: &PiDetection) -> Result<Self> {
        Ok(Self {
            config: RunnerConfig {
                pi_path: detection.executable_path.clone(),
                ..Default::default()
            },
        })
    }

    /// Run a single subagent task
    pub async fn run(
        &self,
        agent_type: PiAgentType,
        task: &str,
        prompt: Option<&str>,
    ) -> Result<SubagentResult> {
        self.run_with_stream(agent_type, task, prompt, |_| {}).await
    }

    /// Run with streaming callback
    pub async fn run_with_stream<F>(
        &self,
        agent_type: PiAgentType,
        task: &str,
        prompt: Option<&str>,
        mut stream_callback: F,
    ) -> Result<SubagentResult>
    where
        F: FnMut(StreamEvent),
    {
        let start_time = Instant::now();
        let agent_name = format!("{:?}", agent_type);
        let agent_type_str = format!("{:?}", agent_type);
        let task = task.to_string();

        // Emit start event
        stream_callback(StreamEvent::start(format!("Starting {} task", agent_name)));

        let mut last_error = None;

        for attempt in 0..self.config.max_retries {
            if attempt > 0 {
                // Emit retry event
                stream_callback(StreamEvent::progress(
                    format!("Retry attempt {}/{}", attempt, self.config.max_retries),
                    Some(attempt.to_string()),
                ));

                // Exponential backoff
                let backoff = Duration::from_millis(100 * 2_u64.pow(attempt as u32 - 1));
                tokio::time::sleep(backoff).await;
            }

            let mut cmd = self.build_command(agent_type, prompt);
            cmd.arg(&task);

            match self.execute_with_timeout(&mut cmd, &mut stream_callback).await {
                Ok((out, code)) => {
                    if code == 0 {
                        // Success
                        let duration = start_time.elapsed();
                        let result = SubagentResult::success(
                            task.clone(),
                            agent_name,
                            agent_type_str,
                            out,
                            duration,
                        );

                        // Emit complete event
                        stream_callback(StreamEvent::complete("Task completed".to_string()));

                        return Ok(result);
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        // All retries exhausted
        let duration = start_time.elapsed();
        let error_msg = last_error
            .as_ref()
            .map(|e| e.to_string())
            .unwrap_or_else(|| "Unknown error".to_string());

        stream_callback(StreamEvent::error(format!("Task failed: {}", error_msg)));

        Ok(SubagentResult::failure(
            task,
            agent_name,
            agent_type_str,
            error_msg,
            duration,
        ))
    }

    /// Build pi-mono command for execution
    fn build_command(
        &self,
        agent_type: PiAgentType,
        prompt: Option<&str>,
    ) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(&self.config.pi_path);

        // Add provider if specified
        if let Some(ref provider) = self.config.provider {
            cmd.arg("--provider");
            cmd.arg(provider);
        }

        // Add model if specified
        if let Some(ref model) = self.config.model {
            cmd.arg("--model");
            cmd.arg(model);
        }

        // Add subagent command
        cmd.arg("subagent");

        // Add agent type
        let agent_arg = match agent_type {
            PiAgentType::Scout => "scout",
            PiAgentType::Planner => "planner",
            PiAgentType::Reviewer => "reviewer",
            PiAgentType::Worker => "worker",
        };
        cmd.arg(agent_arg);

        // Add prompt if specified
        if let Some(prompt_text) = prompt {
            cmd.arg("--prompt");
            cmd.arg(prompt_text);
        }

        cmd
    }

    /// Execute command with timeout
    async fn execute_with_timeout<F>(
        &self,
        cmd: &mut tokio::process::Command,
        stream_callback: &mut F,
    ) -> Result<(String, i32)>
    where
        F: FnMut(StreamEvent),
    {
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let timeout_duration = self.config.timeout;

        let result = tokio::time::timeout(timeout_duration, cmd.output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                if !stderr.is_empty() {
                    stream_callback(StreamEvent::error(stderr.clone()));
                }

                if exit_code != 0 {
                    return Err(Error::Execution(crate::error::ExecutionError::NonZeroExit {
                        command: format!("{:?}", cmd),
                        exit_code,
                        stderr: if stderr.is_empty() { None } else { Some(stderr) },
                    }));
                }

                Ok((stdout, exit_code))
            }
            Ok(Err(e)) => Err(Error::Io(e)),
            Err(_) => Err(Error::Execution(crate::error::ExecutionError::Timeout {
                command: format!("{:?}", cmd),
                timeout_secs: timeout_duration.as_secs(),
            })),
        }
    }
}

impl Default for SubagentRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Create a mock pi executable for testing
    fn create_mock_pi(success: bool) -> PathBuf {
        let mut temp_file = NamedTempFile::new().unwrap();

        // Write a simple shell script that simulates pi behavior
        let script = if success {
            r#"#!/bin/bash
# Mock pi executor - simulates successful execution
shift  # Remove the script name from args

# Parse arguments
SUBAGENT_CMD=""
AGENT_TYPE=""
TASK=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --provider)
            PROVIDER="$2"
            shift 2
            ;;
        --model)
            MODEL="$2"
            shift 2
            ;;
        subagent)
            SUBAGENT_CMD="subagent"
            shift
            ;;
        scout|planner|reviewer|worker)
            AGENT_TYPE="$1"
            shift
            ;;
        --prompt)
            PROMPT="$2"
            shift 2
            ;;
        *)
            TASK="$1"
            shift
            ;;
    esac
done

# Output mock result based on agent type
case "$AGENT_TYPE" in
    scout)
        echo "Scout analysis complete for: $TASK"
        ;;
    planner)
        echo "Plan created for: $TASK"
        ;;
    reviewer)
        echo "Review complete for: $TASK"
        ;;
    worker)
        echo "Work completed for: $TASK"
        ;;
    *)
        echo "Unknown agent type"
        exit 1
        ;;
esac

exit 0
"#
        } else {
            r#"#!/bin/bash
# Mock pi executor - simulates failure
echo "Mock execution failed" >&2
exit 1
"#
        };

        writeln!(temp_file, "{}", script).unwrap();

        // Make it executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = temp_file.as_file().metadata().unwrap().permissions();
            perms.set_mode(0o755);
            temp_file.as_file().set_permissions(perms).unwrap();
        }

        // Close the file and persist it so we can execute it
        let path = temp_file.path().to_path_buf();
        let _ = temp_file.keep(); // Keep the file after NamedTempFile is dropped
        path
    }

    #[test]
    fn test_runner_config_default() {
        let config = RunnerConfig::default();
        assert_eq!(config.pi_path, PathBuf::from("pi"));
        assert_eq!(config.timeout, Duration::from_secs(300));
        assert!(config.provider.is_none());
        assert!(config.model.is_none());
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_runner_config_custom() {
        let config = RunnerConfig {
            pi_path: PathBuf::from("/custom/path/pi"),
            timeout: Duration::from_secs(600),
            provider: Some("anthropic".to_string()),
            model: Some("claude-3".to_string()),
            max_retries: 5,
        };

        assert_eq!(config.pi_path, PathBuf::from("/custom/path/pi"));
        assert_eq!(config.timeout, Duration::from_secs(600));
        assert_eq!(config.provider, Some("anthropic".to_string()));
        assert_eq!(config.model, Some("claude-3".to_string()));
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_subagent_runner_new() {
        let runner = SubagentRunner::new();
        assert_eq!(runner.config.pi_path, PathBuf::from("pi"));
        assert_eq!(runner.config.timeout, Duration::from_secs(300));
        assert_eq!(runner.config.max_retries, 3);
    }

    #[test]
    fn test_subagent_runner_with_config() {
        let config = RunnerConfig {
            pi_path: PathBuf::from("/test/pi"),
            timeout: Duration::from_secs(120),
            provider: Some("test-provider".to_string()),
            model: Some("test-model".to_string()),
            max_retries: 1,
        };

        let runner = SubagentRunner::with_config(config.clone());
        assert_eq!(runner.config.pi_path, PathBuf::from("/test/pi"));
        assert_eq!(runner.config.timeout, Duration::from_secs(120));
        assert_eq!(runner.config.provider, Some("test-provider".to_string()));
        assert_eq!(runner.config.model, Some("test-model".to_string()));
        assert_eq!(runner.config.max_retries, 1);
    }

    #[test]
    fn test_subagent_runner_from_detection() {
        let detection = PiDetection {
            executable_path: PathBuf::from("/detected/pi"),
            version: Some("0.49.3".to_string()),
            capabilities: Default::default(),
        };

        let runner = SubagentRunner::from_detection(&detection).unwrap();
        assert_eq!(runner.config.pi_path, PathBuf::from("/detected/pi"));
        // Other fields should be default
        assert_eq!(runner.config.timeout, Duration::from_secs(300));
        assert_eq!(runner.config.max_retries, 3);
    }

    #[test]
    fn test_subagent_runner_default() {
        let runner = SubagentRunner::default();
        assert_eq!(runner.config.pi_path, PathBuf::from("pi"));
        assert_eq!(runner.config.timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_build_command_scout() {
        let runner = SubagentRunner::new();
        let _cmd = runner.build_command(PiAgentType::Scout, None);

        // Command building succeeds - actual verification is done in integration tests
        // We can't directly inspect tokio::process::Command internals
    }

    #[test]
    fn test_build_command_planner() {
        let runner = SubagentRunner::new();
        let _cmd = runner.build_command(PiAgentType::Planner, Some("Custom prompt"));

        // Command building succeeds
    }

    #[test]
    fn test_build_command_reviewer() {
        let runner = SubagentRunner::new();
        let _cmd = runner.build_command(PiAgentType::Reviewer, None);

        // Command building succeeds
    }

    #[test]
    fn test_build_command_worker() {
        let runner = SubagentRunner::new();
        let _cmd = runner.build_command(PiAgentType::Worker, None);

        // Command building succeeds
    }

    #[test]
    fn test_build_command_with_provider() {
        let config = RunnerConfig {
            provider: Some("anthropic".to_string()),
            ..Default::default()
        };
        let runner = SubagentRunner::with_config(config);
        let _cmd = runner.build_command(PiAgentType::Scout, None);

        // Command building succeeds with provider config
    }

    #[test]
    fn test_build_command_with_model() {
        let config = RunnerConfig {
            model: Some("claude-opus-4".to_string()),
            ..Default::default()
        };
        let runner = SubagentRunner::with_config(config);
        let _cmd = runner.build_command(PiAgentType::Worker, Some("Test prompt"));

        // Command building succeeds with model config
    }

    #[tokio::test]
    async fn test_run_success() {
        let mock_pi_path = create_mock_pi(true);
        let config = RunnerConfig {
            pi_path: mock_pi_path,
            timeout: Duration::from_secs(5),
            ..Default::default()
        };

        let runner = SubagentRunner::with_config(config);
        let result = runner
            .run(PiAgentType::Scout, "test task", None)
            .await
            .unwrap();

        assert!(result.is_success());
        assert_eq!(result.task, "test task");
        assert!(result.output.contains("Scout analysis complete"));
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_run_failure() {
        let mock_pi_path = create_mock_pi(false);
        let config = RunnerConfig {
            pi_path: mock_pi_path,
            max_retries: 1, // Only try once for this test
            ..Default::default()
        };

        let runner = SubagentRunner::with_config(config);
        let result = runner
            .run(PiAgentType::Scout, "test task", None)
            .await
            .unwrap();

        assert!(result.is_failure());
        assert_eq!(result.task, "test task");
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_run_with_stream_callback() {
        let mock_pi_path = create_mock_pi(true);
        let config = RunnerConfig {
            pi_path: mock_pi_path,
            timeout: Duration::from_secs(5),
            ..Default::default()
        };

        let runner = SubagentRunner::with_config(config);
        let mut events = Vec::new();

        let result = runner
            .run_with_stream(
                PiAgentType::Worker,
                "streamed task",
                None,
                |event| events.push(event),
            )
            .await
            .unwrap();

        assert!(result.is_success());
        assert!(events.len() > 0);

        // Verify we got start and complete events
        let event_types: Vec<_> = events.iter().map(|e| e.event_type.clone()).collect();
        assert!(event_types.contains(&crate::execution::StreamEventType::Start));
        assert!(event_types.contains(&crate::execution::StreamEventType::Complete));
    }

    #[tokio::test]
    async fn test_run_all_agent_types() {
        let mock_pi_path = create_mock_pi(true);
        let config = RunnerConfig {
            pi_path: mock_pi_path,
            timeout: Duration::from_secs(5),
            ..Default::default()
        };

        let runner = SubagentRunner::with_config(config);

        // Test Scout
        let scout_result = runner
            .run(PiAgentType::Scout, "scout task", None)
            .await
            .unwrap();
        assert!(scout_result.is_success());
        assert!(scout_result.output.contains("Scout"));

        // Test Planner
        let planner_result = runner
            .run(PiAgentType::Planner, "planner task", None)
            .await
            .unwrap();
        assert!(planner_result.is_success());
        assert!(planner_result.output.contains("Plan"));

        // Test Reviewer
        let reviewer_result = runner
            .run(PiAgentType::Reviewer, "reviewer task", None)
            .await
            .unwrap();
        assert!(reviewer_result.is_success());
        assert!(reviewer_result.output.contains("Review"));

        // Test Worker
        let worker_result = runner
            .run(PiAgentType::Worker, "worker task", None)
            .await
            .unwrap();
        assert!(worker_result.is_success());
        assert!(worker_result.output.contains("Work"));
    }

    #[tokio::test]
    async fn test_run_with_prompt() {
        let mock_pi_path = create_mock_pi(true);
        let config = RunnerConfig {
            pi_path: mock_pi_path,
            timeout: Duration::from_secs(5),
            ..Default::default()
        };

        let runner = SubagentRunner::with_config(config);
        let result = runner
            .run(
                PiAgentType::Scout,
                "test task",
                Some("Custom prompt context"),
            )
            .await
            .unwrap();

        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_retry_logic() {
        let mock_pi_path = create_mock_pi(false);
        let config = RunnerConfig {
            pi_path: mock_pi_path,
            max_retries: 3,
            timeout: Duration::from_secs(1),
            ..Default::default()
        };

        let runner = SubagentRunner::with_config(config);
        let result = runner
            .run(PiAgentType::Scout, "failing task", None)
            .await
            .unwrap();

        assert!(result.is_failure());
        // Should have tried max_retries times
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        // Create a mock pi that sleeps longer than the timeout
        let mut temp_file = NamedTempFile::new().unwrap();

        writeln!(temp_file, "#!/bin/bash").unwrap();
        writeln!(temp_file, "sleep 10").unwrap();
        writeln!(temp_file, "echo 'Done'").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = temp_file.as_file().metadata().unwrap().permissions();
            perms.set_mode(0o755);
            temp_file.as_file().set_permissions(perms).unwrap();
        }

        let mock_path = temp_file.path().to_path_buf();
        let _ = temp_file.keep();

        let config = RunnerConfig {
            pi_path: mock_path,
            timeout: Duration::from_millis(100), // Very short timeout
            max_retries: 1,
            ..Default::default()
        };

        let runner = SubagentRunner::with_config(config);
        let result = runner
            .run(PiAgentType::Scout, "timeout test", None)
            .await
            .unwrap();

        assert!(result.is_failure());
        assert!(result.error.unwrap().contains("timed out"));
    }

    #[test]
    fn test_runner_clone_config() {
        let config = RunnerConfig {
            pi_path: PathBuf::from("/test/pi"),
            timeout: Duration::from_secs(100),
            provider: Some("test".to_string()),
            model: Some("model".to_string()),
            max_retries: 2,
        };

        let cloned = config.clone();
        assert_eq!(cloned.pi_path, config.pi_path);
        assert_eq!(cloned.timeout, config.timeout);
        assert_eq!(cloned.provider, config.provider);
        assert_eq!(cloned.model, config.model);
        assert_eq!(cloned.max_retries, config.max_retries);
    }

    #[tokio::test]
    async fn test_result_duration_is_measured() {
        let mock_pi_path = create_mock_pi(true);
        let config = RunnerConfig {
            pi_path: mock_pi_path,
            timeout: Duration::from_secs(5),
            ..Default::default()
        };

        let runner = SubagentRunner::with_config(config);
        let result = runner
            .run(PiAgentType::Scout, "timing test", None)
            .await
            .unwrap();

        assert!(result.is_success());
        assert!(result.duration.as_millis() > 0);
    }

    #[tokio::test]
    async fn test_stream_events_in_result() {
        let mock_pi_path = create_mock_pi(true);
        let config = RunnerConfig {
            pi_path: mock_pi_path,
            timeout: Duration::from_secs(5),
            ..Default::default()
        };

        let runner = SubagentRunner::with_config(config);

        let result = runner
            .run_with_stream(
                PiAgentType::Worker,
                "event test",
                Some("test prompt"),
                |_| {},
            )
            .await
            .unwrap();

        assert!(result.is_success());
        // Note: events are collected via callback, not in result.events
        // The runner implementation uses callback for streaming
    }

    #[tokio::test]
    async fn test_error_output_in_stream() {
        let mock_pi_path = create_mock_pi(false);
        let config = RunnerConfig {
            pi_path: mock_pi_path,
            max_retries: 1,
            ..Default::default()
        };

        let runner = SubagentRunner::with_config(config);
        let mut got_error = false;

        let _result = runner
            .run_with_stream(
                PiAgentType::Scout,
                "error test",
                None,
                |event| {
                    if event.event_type == crate::execution::StreamEventType::Error {
                        got_error = true;
                    }
                },
            )
            .await
            .unwrap();

        // Should get error event when command fails
        assert!(got_error);
    }

    #[tokio::test]
    async fn test_custom_pi_path_from_detection() {
        let detection = PiDetection {
            executable_path: PathBuf::from("/custom/pi/path"),
            version: None,
            capabilities: Default::default(),
        };

        let runner = SubagentRunner::from_detection(&detection).unwrap();
        assert_eq!(runner.config.pi_path, PathBuf::from("/custom/pi/path"));
    }
}
