//! Orchestrate data models
//!
//! Track and task tree structures for Maestro orchestration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Track status marker in tracks.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackStatus {
    Pending,    // [ ]
    InProgress, // [~]
    Completed,  // [x]
}

impl TrackStatus {
    pub fn from_marker(marker: &str) -> Option<Self> {
        match marker.trim() {
            "[ ]" | " " => Some(TrackStatus::Pending),
            "[~]" => Some(TrackStatus::InProgress),
            "[x]" => Some(TrackStatus::Completed),
            _ => None,
        }
    }

    pub fn to_marker(self) -> &'static str {
        match self {
            TrackStatus::Pending => "[ ]",
            TrackStatus::InProgress => "[~]",
            TrackStatus::Completed => "[x]",
        }
    }

    pub fn is_actionable(&self) -> bool {
        matches!(self, TrackStatus::Pending | TrackStatus::InProgress)
    }
}

/// Task dependency relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDependency {
    pub task_id: String,
    pub dependency_type: TaskDependencyType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskDependencyType {
    Hard, // Must complete before this task can start
    Soft, // Should complete, but not blocking
}

/// A single task in a track plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TrackStatus,
    pub dependencies: Vec<TaskDependency>,
    pub description: String,
    pub subtasks: Vec<Task>,
    pub notes: Option<String>,
    pub line_number: usize,
}

impl Task {
    /// Check if this task is actionable (all hard dependencies satisfied)
    pub fn is_actionable(&self, completed_tasks: &HashMap<String, bool>) -> bool {
        if !self.status.is_actionable() {
            return false;
        }

        self.dependencies
            .iter()
            .filter(|d| d.dependency_type == TaskDependencyType::Hard)
            .all(|d| completed_tasks.get(&d.task_id).copied().unwrap_or(false))
    }

    /// Check if this task is blocked (has unsatisfied hard dependencies)
    pub fn is_blocked(&self, completed_tasks: &HashMap<String, bool>) -> bool {
        self.dependencies
            .iter()
            .filter(|d| d.dependency_type == TaskDependencyType::Hard)
            .any(|d| !completed_tasks.get(&d.task_id).copied().unwrap_or(false))
    }

    /// Get all descendant task IDs (including subtasks recursively)
    pub fn all_task_ids(&self) -> Vec<String> {
        let mut ids = vec![self.id.clone()];
        for subtask in &self.subtasks {
            ids.extend(subtask.all_task_ids());
        }
        ids
    }
}

/// Track metadata from metadata.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackMetadata {
    pub track_id: String,
    #[serde(rename = "type")]
    pub track_type: TrackType,
    pub status: TrackStatus,
    pub created_at: String,
    pub updated_at: String,
    pub description: String,
    pub sub_tracks: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackType {
    Feature,
    Master,
    Refactor,
    Hotfix,
}

/// A Maestro track (from tracks.md)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub description: String,
    pub status: TrackStatus,
    pub link_path: PathBuf,
    pub metadata: Option<TrackMetadata>,
    pub plan: Option<TrackPlan>,
}

impl Track {
    /// Check if this is a master track
    pub fn is_master(&self) -> bool {
        self.metadata
            .as_ref()
            .map(|m| m.track_type == TrackType::Master)
            .unwrap_or(false)
    }

    /// Get the highest priority actionable task
    pub fn next_actionable_task(&self) -> Option<&Task> {
        self.plan.as_ref()?.next_actionable_task()
    }
}

/// Track plan parsed from plan.md
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackPlan {
    pub track_id: String,
    pub tasks: Vec<Task>,
    pub phases: Vec<Phase>,
}

impl TrackPlan {
    /// Get the highest priority actionable task
    pub fn next_actionable_task(&self) -> Option<&Task> {
        let completed = self.completed_tasks_map();
        self.find_actionable(&completed)
    }

    fn find_actionable<'a>(&'a self, completed: &HashMap<String, bool>) -> Option<&'a Task> {
        for task in &self.tasks {
            if let Some(actionable) = Self::find_actionable_recursive(task, completed) {
                return Some(actionable);
            }
        }
        None
    }

    fn find_actionable_recursive<'a>(
        task: &'a Task,
        completed: &HashMap<String, bool>,
    ) -> Option<&'a Task> {
        // First check subtasks (depth-first for hierarchy)
        for subtask in &task.subtasks {
            if let Some(actionable) = Self::find_actionable_recursive(subtask, completed) {
                return Some(actionable);
            }
        }

        // Then check this task
        if task.is_actionable(completed) {
            return Some(task);
        }

        None
    }

    fn completed_tasks_map(&self) -> HashMap<String, bool> {
        let mut map = HashMap::new();
        for task in &self.tasks {
            self.collect_completed(task, &mut map);
        }
        map
    }

    fn collect_completed(&self, task: &Task, map: &mut HashMap<String, bool>) {
        map.insert(task.id.clone(), task.status == TrackStatus::Completed);
        for subtask in &task.subtasks {
            self.collect_completed(subtask, map);
        }
    }

    /// Get all tasks flattened
    pub fn all_tasks(&self) -> Vec<&Task> {
        let mut tasks = Vec::new();
        for task in &self.tasks {
            self.collect_tasks(task, &mut tasks);
        }
        tasks
    }

    fn collect_tasks<'a>(&'a self, task: &'a Task, tasks: &mut Vec<&'a Task>) {
        tasks.push(task);
        for subtask in &task.subtasks {
            self.collect_tasks(subtask, tasks);
        }
    }
}

/// A phase in the track plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub name: String,
    pub tasks: Vec<String>, // Task IDs
}

/// Loop mode for orchestrate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoopMode {
    Planning,
    Building,
}

/// Agent configuration for running iterations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub tool: String,
    pub model: Option<String>,
    pub dangerous_mode: bool,  // CORRECTED: was "dangerous"
    pub sandbox: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            tool: "claude".to_string(),
            model: None,
            dangerous_mode: false,
            sandbox: false,
        }
    }
}

/// Orchestrate session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: String,
    pub track_id: String,
    pub mode: LoopMode,
    pub agent_config: AgentConfig,
    pub current_iteration: u64,
    pub current_task_id: Option<String>,
    pub started_at: String,
    pub updated_at: String,
    pub status: SessionStatus,
    pub rate_limit: Option<crate::rate_limit::RateLimitState>,
    /// Per-task retry counts (task_id -> retry_count)
    #[serde(default)]
    pub retry_counts: std::collections::HashMap<String, u32>,
    /// Maximum iterations (0 = unlimited)
    #[serde(default)]
    pub max_iterations: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Idle,
    Running,
    /// Transition state before pausing - allows canceling pending pause
    Pausing,
    Paused,
    Completed,
    Failed,
    Interrupted,
    /// Engine is stopping (graceful shutdown in progress)
    Stopping,
}

/// Iteration log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationLog {
    pub iteration: u64,
    pub task_id: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: IterationStatus,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IterationStatus {
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Error strategy for handling failures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorStrategy {
    Retry,
    Skip,
    Abort,
}

impl Default for ErrorStrategy {
    fn default() -> Self {
        Self::Retry
    }
}

/// Orchestrate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrateConfig {
    pub max_retries: u32,
    pub error_strategy: ErrorStrategy,
    /// Base backoff in milliseconds for task-level retries (exponential)
    #[serde(default = "default_retry_backoff_base_ms")]
    pub retry_backoff_base_ms: u64,
    pub context_budget: usize, // Max tokens for context
    pub iteration_timeout_secs: u64,
    pub enable_leindex: bool,
    pub data_dir: PathBuf,
    // Rate-limit detection and recovery
    pub enable_rate_limit_detection: bool,
    pub rate_limit_max_retries: u32,
    pub rate_limit_backoff_base_secs: u64,
    pub rate_limit_backoff_max_secs: u64,
    /// LSP diagnostic validation after agent edits
    #[serde(default)]
    pub lsp_diagnostics: LspDiagnosticConfig,
}

/// LSP diagnostic configuration for post-edit validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDiagnosticConfig {
    /// Enable LSP diagnostic checking after agent edits
    #[serde(default)]
    pub enabled: bool,
    /// Fail iteration if LSP errors are found
    #[serde(default = "default_fail_on_errors")]
    pub fail_on_errors: bool,
    /// Fail iteration if LSP warnings are found
    #[serde(default)]
    pub fail_on_warnings: bool,
    /// Maximum number of diagnostics to display
    #[serde(default = "default_max_diagnostics")]
    pub max_diagnostics: usize,
    /// Timeout in seconds for LSP diagnostic requests
    #[serde(default = "default_diagnostic_timeout_secs")]
    pub timeout_secs: u64,
    /// File patterns to include in diagnostic checks
    #[serde(default)]
    pub include_patterns: Vec<String>,
    /// File patterns to exclude from diagnostic checks
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
    /// Directories to skip (e.g., "target", "node_modules")
    #[serde(default)]
    pub exclude_dirs: Vec<String>,
}

impl Default for LspDiagnosticConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fail_on_errors: true,
            fail_on_warnings: false,
            max_diagnostics: 50,
            timeout_secs: 10,
            include_patterns: vec![
                "**/*.rs".to_string(),
                "**/*.py".to_string(),
                "**/*.ts".to_string(),
                "**/*.tsx".to_string(),
                "**/*.js".to_string(),
                "**/*.jsx".to_string(),
                "**/*.go".to_string(),
                "**/*.java".to_string(),
            ],
            exclude_patterns: vec![
                "**/.*".to_string(),
                "**/*.generated.rs".to_string(),
            ],
            exclude_dirs: vec![
                "target".to_string(),
                "node_modules".to_string(),
                ".git".to_string(),
                "vendor".to_string(),
                "dist".to_string(),
                "build".to_string(),
                "__pycache__".to_string(),
                ".venv".to_string(),
                "venv".to_string(),
                ".cargo".to_string(),
            ],
        }
    }
}

fn default_fail_on_errors() -> bool {
    true
}

fn default_max_diagnostics() -> usize {
    50
}

fn default_diagnostic_timeout_secs() -> u64 {
    10
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        "**/.*".to_string(),
        "**/*.generated.rs".to_string(),
    ]
}

impl Default for OrchestrateConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        Self {
            max_retries: 3,
            error_strategy: ErrorStrategy::Retry,
            retry_backoff_base_ms: 5_000,
            context_budget: 50000,
            iteration_timeout_secs: 300, // 5 minutes
            enable_leindex: true,
            data_dir: PathBuf::from(home).join(".maestro").join("orchestrate"),
            enable_rate_limit_detection: true,
            rate_limit_max_retries: 5,
            rate_limit_backoff_base_secs: 1,
            rate_limit_backoff_max_secs: 300,
            lsp_diagnostics: LspDiagnosticConfig::default(),
        }
    }
}

fn default_retry_backoff_base_ms() -> u64 {
    5_000
}
