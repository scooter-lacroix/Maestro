//! # Execution module for Pi-Mono integration
//!
//! This module handles execution of Pi-Mono agents and tasks.
//!
//! ## Example
//!
//! ```rust
//! use maestro_pi_mono::execution::{Executor, ExecutorConfig};
//!
//! // Create an executor with default configuration
//! let executor = Executor::default();
//!
//! // Or create with custom configuration
//! let config = ExecutorConfig {
//!     timeout_secs: 600,
//!     max_retries: 5,
//! };
//! let executor = Executor::new(config);
//! ```

pub mod runner;

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Result of a Pi-Mono execution.
///
/// # Examples
///
/// Creating a successful result:
///
/// ```rust
/// use maestro_pi_mono::execution::ExecutionResult;
///
/// let result = ExecutionResult::success("Operation completed".to_string());
/// assert!(result.success);
/// assert_eq!(result.output, "Operation completed");
/// assert!(result.error.is_none());
/// ```
///
/// Creating a failed result:
///
/// ```rust
/// use maestro_pi_mono::execution::ExecutionResult;
///
/// let result = ExecutionResult::failure("Connection failed".to_string());
/// assert!(!result.success);
/// assert!(result.output.is_empty());
/// assert_eq!(result.error, Some("Connection failed".to_string()));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether the execution was successful
    pub success: bool,

    /// Output from the execution
    pub output: String,

    /// Error message, if any
    pub error: Option<String>,
}

impl ExecutionResult {
    /// Create a successful execution result.
    ///
    /// # Arguments
    ///
    /// * `output` - The output string from the successful execution
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::ExecutionResult;
    ///
    /// let result = ExecutionResult::success("Task completed successfully".to_string());
    /// assert!(result.success);
    /// assert!(result.error.is_none());
    /// ```
    pub fn success(output: String) -> Self {
        Self {
            success: true,
            output,
            error: None,
        }
    }

    /// Create a failed execution result.
    ///
    /// # Arguments
    ///
    /// * `error` - The error message describing the failure
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::ExecutionResult;
    ///
    /// let result = ExecutionResult::failure("Timeout occurred".to_string());
    /// assert!(!result.success);
    /// assert_eq!(result.error, Some("Timeout occurred".to_string()));
    /// ```
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error),
        }
    }
}

/// Executor for Pi-Mono operations.
///
/// Executes commands with timeout and retry support.
///
/// # Examples
///
/// Creating an executor with default configuration:
///
/// ```rust
/// use maestro_pi_mono::execution::Executor;
///
/// let executor = Executor::default();
/// ```
///
/// Creating an executor with custom configuration:
///
/// ```rust
/// use maestro_pi_mono::execution::{Executor, ExecutorConfig};
///
/// let config = ExecutorConfig {
///     timeout_secs: 600,
///     max_retries: 5,
/// };
/// let executor = Executor::new(config);
/// ```
#[derive(Debug, Clone)]
pub struct Executor {
    /// Configuration for the executor
    #[allow(dead_code)]
    config: ExecutorConfig,
}

impl Default for Executor {
    /// Creates an executor with default configuration:
    /// - 300 second timeout
    /// - 3 maximum retries
    fn default() -> Self {
        Self {
            config: ExecutorConfig::default(),
        }
    }
}

impl Executor {
    /// Create a new executor with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The executor configuration to use
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::{Executor, ExecutorConfig};
    ///
    /// let config = ExecutorConfig {
    ///     timeout_secs: 120,
    ///     max_retries: 1,
    /// };
    /// let executor = Executor::new(config);
    /// ```
    pub fn new(config: ExecutorConfig) -> Self {
        Self { config }
    }

    /// Execute a command with timeout and retry support.
    ///
    /// The command string is parsed into arguments using shell-like splitting.
    /// For complex shell operations, use a shell explicitly.
    ///
    /// # Arguments
    ///
    /// * `command` - The command string to execute
    ///
    /// # Returns
    ///
    /// A `Result` containing the `ExecutionResult` or an error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::Executor;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let executor = Executor::default();
    /// let result = executor.execute("echo hello").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute(&self, command: &str) -> anyhow::Result<ExecutionResult> {
        use crate::error::{ExecutionError, Error as PiError};
        use tokio::process::Command;

        // Parse command into arguments
        let parts = shell_words::split(command)
            .map_err(|e| anyhow::anyhow!("Invalid command syntax: {}", e))?;

        if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty command"));
        }

        let binary = &parts[0];
        let args = &parts[1..];

        let timeout_duration = Duration::from_secs(self.config.timeout_secs);
        let mut last_error = None;

        // Retry loop with exponential backoff
        for attempt in 0..=self.config.max_retries {
            let result = tokio::time::timeout(
                timeout_duration,
                Command::new(binary)
                    .args(args)
                    .output()
            ).await;

            match result {
                Ok(Ok(output)) => {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        return Ok(ExecutionResult::success(stdout));
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        let code = output.status.code().unwrap_or(-1);
                        last_error = Some(anyhow::Error::new(PiError::Execution(
                            ExecutionError::NonZeroExit {
                                command: command.to_string(),
                                exit_code: code,
                                stderr: if stderr.is_empty() { None } else { Some(stderr) },
                            }
                        )));
                    }
                }
                Ok(Err(e)) => {
                    last_error = Some(e.into());
                }
                Err(_) => {
                    last_error = Some(anyhow::Error::new(PiError::Execution(
                        ExecutionError::Timeout {
                            command: command.to_string(),
                            timeout_secs: self.config.timeout_secs,
                        }
                    )));
                }
            }

            // Exponential backoff before retry
            if attempt < self.config.max_retries {
                let backoff = Duration::from_millis(100 * 2u64.pow(attempt as u32));
                tokio::time::sleep(backoff).await;
            }
        }

        // All retries exhausted
        let error_msg = last_error
            .as_ref()
            .map(|e| e.to_string())
            .unwrap_or_else(|| "Unknown error".to_string());

        Err(anyhow::anyhow!(
            "Command failed after {} attempts: {}",
            self.config.max_retries + 1,
            error_msg
        ))
    }
}

/// Configuration for the executor.
///
/// # Examples
///
/// Using default configuration:
///
/// ```rust
/// use maestro_pi_mono::execution::ExecutorConfig;
///
/// let config = ExecutorConfig::default();
/// assert_eq!(config.timeout_secs, 300);
/// assert_eq!(config.max_retries, 3);
/// ```
///
/// Custom configuration:
///
/// ```rust
/// use maestro_pi_mono::execution::ExecutorConfig;
///
/// let config = ExecutorConfig {
///     timeout_secs: 600,
///     max_retries: 5,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// Timeout for executions in seconds
    pub timeout_secs: u64,

    /// Maximum number of retries
    pub max_retries: usize,
}

impl Default for ExecutorConfig {
    /// Creates default executor configuration:
    /// - 300 second (5 minute) timeout
    /// - 3 maximum retries
    fn default() -> Self {
        Self {
            timeout_secs: 300,
            max_retries: 3,
        }
    }
}

/// Usage metrics from subagent execution.
///
/// # Examples
///
/// Creating usage metrics:
///
/// ```rust
/// use maestro_pi_mono::execution::UsageMetrics;
/// use std::time::Duration;
///
/// let metrics = UsageMetrics {
///     tokens_input: 1000,
///     tokens_output: 500,
///     tokens_total: 1500,
///     cost_estimate_usd: Some(0.003),
///     duration: Duration::from_secs(10),
/// };
/// assert_eq!(metrics.tokens_input, 1000);
/// assert_eq!(metrics.tokens_total, 1500);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageMetrics {
    /// Number of input tokens consumed
    pub tokens_input: u64,

    /// Number of output tokens generated
    pub tokens_output: u64,

    /// Total tokens used (input + output)
    pub tokens_total: u64,

    /// Estimated cost in USD (if available)
    pub cost_estimate_usd: Option<f64>,

    /// Duration of the execution
    pub duration: Duration,
}

impl UsageMetrics {
    /// Create new usage metrics.
    ///
    /// # Arguments
    ///
    /// * `tokens_input` - Number of input tokens
    /// * `tokens_output` - Number of output tokens
    /// * `cost_estimate_usd` - Optional cost estimate in USD
    /// * `duration` - Execution duration
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::UsageMetrics;
    /// use std::time::Duration;
    ///
    /// let metrics = UsageMetrics::new(
    ///     1000,
    ///     500,
    ///     Some(0.003),
    ///     Duration::from_secs(10)
    /// );
    /// assert_eq!(metrics.tokens_total, 1500);
    /// ```
    pub fn new(
        tokens_input: u64,
        tokens_output: u64,
        cost_estimate_usd: Option<f64>,
        duration: Duration,
    ) -> Self {
        let tokens_total = tokens_input + tokens_output;
        Self {
            tokens_input,
            tokens_output,
            tokens_total,
            cost_estimate_usd,
            duration,
        }
    }

    /// Calculate cost per million tokens.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::UsageMetrics;
    /// use std::time::Duration;
    ///
    /// let metrics = UsageMetrics::new(
    ///     1000,
    ///     500,
    ///     Some(0.003),
    ///     Duration::from_secs(10)
    /// );
    /// let cost_per_million = metrics.cost_per_million_tokens();
    /// assert_eq!(cost_per_million, Some(2.0));
    /// ```
    pub fn cost_per_million_tokens(&self) -> Option<f64> {
        self.cost_estimate_usd
            .map(|cost| (cost / self.tokens_total as f64) * 1_000_000.0)
    }

    /// Calculate tokens per second.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::UsageMetrics;
    /// use std::time::Duration;
    ///
    /// let metrics = UsageMetrics::new(
    ///     1000,
    ///     500,
    ///     Some(0.003),
    ///     Duration::from_secs(10)
    /// );
    /// let tps = metrics.tokens_per_second();
    /// assert_eq!(tps, 150.0);
    /// ```
    pub fn tokens_per_second(&self) -> f64 {
        let secs = self.duration.as_secs_f64();
        if secs > 0.0 {
            self.tokens_total as f64 / secs
        } else {
            0.0
        }
    }
}

/// Stream event types.
///
/// # Examples
///
/// Creating stream event types:
///
/// ```rust
/// use maestro_pi_mono::execution::StreamEventType;
///
/// let start = StreamEventType::Start;
/// let progress = StreamEventType::Progress;
/// let complete = StreamEventType::Complete;
/// assert_eq!(start, StreamEventType::Start);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StreamEventType {
    /// Execution started
    Start,

    /// Progress update
    Progress,

    /// Data chunk received
    Data,

    /// Error occurred
    Error,

    /// Execution completed
    Complete,
}

/// Real-time stream event.
///
/// # Examples
///
/// Creating a stream event:
///
/// ```rust
/// use maestro_pi_mono::execution::{StreamEvent, StreamEventType};
/// use std::time::SystemTime;
///
/// let event = StreamEvent {
///     timestamp: SystemTime::now(),
///     event_type: StreamEventType::Progress,
///     content: "Processing...".to_string(),
///     metadata: Some("50%".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    /// When the event occurred
    pub timestamp: std::time::SystemTime,

    /// Type of event
    pub event_type: StreamEventType,

    /// Event content
    pub content: String,

    /// Optional metadata
    pub metadata: Option<String>,
}

impl StreamEvent {
    /// Create a new stream event.
    ///
    /// # Arguments
    ///
    /// * `event_type` - Type of the event
    /// * `content` - Event content
    /// * `metadata` - Optional metadata
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::{StreamEvent, StreamEventType};
    ///
    /// let event = StreamEvent::new(
    ///     StreamEventType::Progress,
    ///     "Processing...".to_string(),
    ///     Some("50%".to_string())
    /// );
    /// assert_eq!(event.content, "Processing...");
    /// ```
    pub fn new(event_type: StreamEventType, content: String, metadata: Option<String>) -> Self {
        Self {
            timestamp: std::time::SystemTime::now(),
            event_type,
            content,
            metadata,
        }
    }

    /// Create a start event.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::StreamEvent;
    ///
    /// let event = StreamEvent::start("Starting task...".to_string());
    /// ```
    pub fn start(content: String) -> Self {
        Self::new(StreamEventType::Start, content, None)
    }

    /// Create a progress event.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::StreamEvent;
    ///
    /// let event = StreamEvent::progress("50% complete".to_string(), Some("50".to_string()));
    /// ```
    pub fn progress(content: String, metadata: Option<String>) -> Self {
        Self::new(StreamEventType::Progress, content, metadata)
    }

    /// Create a data event.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::StreamEvent;
    ///
    /// let event = StreamEvent::data("Received data chunk".to_string());
    /// ```
    pub fn data(content: String) -> Self {
        Self::new(StreamEventType::Data, content, None)
    }

    /// Create an error event.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::StreamEvent;
    ///
    /// let event = StreamEvent::error("Connection failed".to_string());
    /// ```
    pub fn error(content: String) -> Self {
        Self::new(StreamEventType::Error, content, None)
    }

    /// Create a complete event.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::StreamEvent;
    ///
    /// let event = StreamEvent::complete("Task finished".to_string());
    /// ```
    pub fn complete(content: String) -> Self {
        Self::new(StreamEventType::Complete, content, None)
    }
}

/// Detailed subagent execution result.
///
/// # Examples
///
/// Creating a successful subagent result:
///
/// ```rust
/// use maestro_pi_mono::execution::SubagentResult;
/// use std::time::Duration;
///
/// let result = SubagentResult::success(
///     "Analyze code".to_string(),
///     "agent-001".to_string(),
///     "analyzer".to_string(),
///     "Analysis complete".to_string(),
///     Duration::from_secs(5)
/// );
/// assert!(result.is_success());
/// assert_eq!(result.task, "Analyze code");
/// ```
///
/// Creating a failed subagent result:
///
/// ```rust
/// use maestro_pi_mono::execution::SubagentResult;
/// use std::time::Duration;
///
/// let result = SubagentResult::failure(
///     "Analyze code".to_string(),
///     "agent-001".to_string(),
///     "analyzer".to_string(),
///     "Timeout error".to_string(),
///     Duration::from_secs(10)
/// );
/// assert!(!result.is_success());
/// assert_eq!(result.error, Some("Timeout error".to_string()));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentResult {
    /// Whether execution succeeded
    pub success: bool,

    /// Task description
    pub task: String,

    /// Agent identifier
    pub agent: String,

    /// Agent type (e.g., "analyzer", "coder", "reviewer")
    pub agent_type: String,

    /// Output from execution
    pub output: String,

    /// Error message if failed
    pub error: Option<String>,

    /// Exit code if available
    pub exit_code: Option<i32>,

    /// Execution duration
    pub duration: Duration,

    /// Usage metrics if available
    pub usage: Option<UsageMetrics>,

    /// Stream events captured during execution
    pub events: Vec<StreamEvent>,
}

impl SubagentResult {
    /// Create a successful subagent result.
    ///
    /// # Arguments
    ///
    /// * `task` - Task description
    /// * `agent` - Agent identifier
    /// * `agent_type` - Type of agent
    /// * `output` - Execution output
    /// * `duration` - Execution duration
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::SubagentResult;
    /// use std::time::Duration;
    ///
    /// let result = SubagentResult::success(
    ///     "Fix bug".to_string(),
    ///     "agent-002".to_string(),
    ///     "coder".to_string(),
    ///     "Bug fixed".to_string(),
    ///     Duration::from_secs(8)
    /// );
    /// assert!(result.is_success());
    /// assert!(result.error.is_none());
    /// ```
    pub fn success(
        task: String,
        agent: String,
        agent_type: String,
        output: String,
        duration: Duration,
    ) -> Self {
        Self {
            success: true,
            task,
            agent,
            agent_type,
            output,
            error: None,
            exit_code: Some(0),
            duration,
            usage: None,
            events: Vec::new(),
        }
    }

    /// Create a failed subagent result.
    ///
    /// # Arguments
    ///
    /// * `task` - Task description
    /// * `agent` - Agent identifier
    /// * `agent_type` - Type of agent
    /// * `error` - Error message
    /// * `duration` - Execution duration
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::SubagentResult;
    /// use std::time::Duration;
    ///
    /// let result = SubagentResult::failure(
    ///     "Fix bug".to_string(),
    ///     "agent-002".to_string(),
    ///     "coder".to_string(),
    ///     "Compilation failed".to_string(),
    ///     Duration::from_secs(3)
    /// );
    /// assert!(!result.is_success());
    /// assert_eq!(result.error, Some("Compilation failed".to_string()));
    /// ```
    pub fn failure(
        task: String,
        agent: String,
        agent_type: String,
        error: String,
        duration: Duration,
    ) -> Self {
        Self {
            success: false,
            task,
            agent,
            agent_type,
            output: String::new(),
            error: Some(error),
            exit_code: None,
            duration,
            usage: None,
            events: Vec::new(),
        }
    }

    /// Add usage metrics to the result.
    ///
    /// # Arguments
    ///
    /// * `usage` - Usage metrics to add
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::{SubagentResult, UsageMetrics};
    /// use std::time::Duration;
    ///
    /// let result = SubagentResult::success(
    ///     "Task".to_string(),
    ///     "agent-001".to_string(),
    ///     "analyzer".to_string(),
    ///     "Done".to_string(),
    ///     Duration::from_secs(5)
    /// ).with_usage(UsageMetrics::new(
    ///     1000, 500, Some(0.003), Duration::from_secs(5)
    /// ));
    /// assert!(result.usage.is_some());
    /// ```
    pub fn with_usage(mut self, usage: UsageMetrics) -> Self {
        self.usage = Some(usage);
        self
    }

    /// Add a stream event to the result.
    ///
    /// # Arguments
    ///
    /// * `event` - Stream event to add
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::{SubagentResult, StreamEvent};
    /// use std::time::Duration;
    ///
    /// let result = SubagentResult::success(
    ///     "Task".to_string(),
    ///     "agent-001".to_string(),
    ///     "analyzer".to_string(),
    ///     "Done".to_string(),
    ///     Duration::from_secs(5)
    /// ).with_event(StreamEvent::start("Starting".to_string()));
    /// assert_eq!(result.events.len(), 1);
    /// ```
    pub fn with_event(mut self, event: StreamEvent) -> Self {
        self.events.push(event);
        self
    }

    /// Check if execution succeeded.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::SubagentResult;
    /// use std::time::Duration;
    ///
    /// let result = SubagentResult::success(
    ///     "Task".to_string(),
    ///     "agent-001".to_string(),
    ///     "analyzer".to_string(),
    ///     "Done".to_string(),
    ///     Duration::from_secs(5)
    /// );
    /// assert!(result.is_success());
    /// ```
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Check if execution failed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::SubagentResult;
    /// use std::time::Duration;
    ///
    /// let result = SubagentResult::failure(
    ///     "Task".to_string(),
    ///     "agent-001".to_string(),
    ///     "analyzer".to_string(),
    ///     "Error".to_string(),
    ///     Duration::from_secs(5)
    /// );
    /// assert!(result.is_failure());
    /// ```
    pub fn is_failure(&self) -> bool {
        !self.success
    }

    /// Get the number of events.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::SubagentResult;
    /// use std::time::Duration;
    ///
    /// let result = SubagentResult::success(
    ///     "Task".to_string(),
    ///     "agent-001".to_string(),
    ///     "analyzer".to_string(),
    ///     "Done".to_string(),
    ///     Duration::from_secs(5)
    /// );
    /// assert_eq!(result.event_count(), 0);
    /// ```
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Get all error events.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::{SubagentResult, StreamEvent};
    /// use std::time::Duration;
    ///
    /// let result = SubagentResult::success(
    ///     "Task".to_string(),
    ///     "agent-001".to_string(),
    ///     "analyzer".to_string(),
    ///     "Done".to_string(),
    ///     Duration::from_secs(5)
    /// )
    /// .with_event(StreamEvent::error("Warning".to_string()))
    /// .with_event(StreamEvent::data("Data".to_string()));
    ///
    /// let errors = result.error_events();
    /// assert_eq!(errors.len(), 1);
    /// ```
    pub fn error_events(&self) -> Vec<&StreamEvent> {
        self.events
            .iter()
            .filter(|e| e.event_type == StreamEventType::Error)
            .collect()
    }

    /// Get formatted summary of the result.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::execution::SubagentResult;
    /// use std::time::Duration;
    ///
    /// let result = SubagentResult::success(
    ///     "Task".to_string(),
    ///     "agent-001".to_string(),
    ///     "analyzer".to_string(),
    ///     "Done".to_string(),
    ///     Duration::from_secs(5)
    /// );
    /// let summary = result.summary();
    /// assert!(summary.contains("SUCCESS"));
    /// ```
    pub fn summary(&self) -> String {
        let status = if self.success { "SUCCESS" } else { "FAILURE" };
        format!(
            "[{}] {} - {} ({}) in {:?}",
            status, self.task, self.agent, self.agent_type, self.duration
        )
    }
}
