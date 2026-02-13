//! Conductor data models and events
//!
//! Based on Ralph TUI's state machine and execution loop.

use crate::maestro_paths::MaestroProject;
use chrono::{DateTime, Utc};
use leindex_core::orchestrate::model::LoopMode;
use serde::{Deserialize, Serialize};

/// Ralph: RalphStatus → Maestro: ConductorStatus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConductorStatus {
    #[default]
    /// Waiting for user to start
    Ready,
    /// Generic running state
    Running,
    /// Selecting next task from plan
    Selecting,
    /// Agent actively executing task
    Executing,
    /// Pause requested, finishing current iteration
    Pausing,
    /// Waiting to resume
    Paused,
    /// Shutting down
    Stopping,
    /// All tasks in track finished
    Completed,
    /// No more actionable tasks available
    Idle,
    /// Stopped due to error
    Failed,
}

/// Ralph: ActiveAgentState
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveAgentState {
    /// Agent tool name (claude, gemini, qwen, opencode, etc.)
    pub tool: String,
    /// Model identifier (optional)
    pub model: Option<String>,
    /// Why this agent is active
    pub reason: AgentReason,
    /// When this agent became active
    pub since: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentReason {
    #[default]
    /// Configured primary agent
    Primary,
    /// Switched due to rate limit or error
    Fallback,
}

/// Ralph: RateLimitState
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitState {
    /// Primary agent that was rate limited
    pub primary_agent: String,
    /// When the primary was rate limited (None if not limited)
    pub limited_at: Option<DateTime<Utc>>,
    /// Current fallback agent in use (None if using primary)
    pub fallback_agent: Option<String>,
    /// Current retry count for rate limit backoff
    pub retry_count: u32,
    /// When backoff expires (represented as DateTime<Utc> for serialization)
    pub backoff_until: Option<DateTime<Utc>>,
    /// Last rate limit message from agent
    pub last_message: Option<String>,
}

/// Ralph: EngineState → Maestro: ConductorState
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductorState {
    /// Current execution status
    pub status: ConductorStatus,
    /// Session ID
    pub session_id: Option<String>,
    /// Current iteration number (1-based for display)
    pub current_iteration: u64,
    /// Maximum iterations (0 = unlimited)
    pub max_iterations: u64,
    /// Currently selected track
    pub current_track: Option<String>,
    /// Currently executing task
    pub current_task: Option<String>,
    /// Tasks completed this session
    pub tasks_completed: usize,
    /// Total actionable tasks
    pub total_tasks: usize,
    /// Session start time
    pub started_at: Option<DateTime<Utc>>,
    /// Elapsed time in seconds
    pub elapsed_secs: u64,
    /// Current iteration output buffer
    pub current_output: String,
    /// Current iteration stderr buffer  
    pub current_stderr: String,
    /// Active subagents during iteration
    pub subagents: Vec<SubagentState>,
    /// Active agent state
    pub active_agent: Option<ActiveAgentState>,
    /// Rate limit tracking
    pub rate_limit: Option<RateLimitState>,
    /// Last iteration loaded (to avoid full re-reads)
    pub last_poll_iteration: u64,
    /// Last byte offset read from iterations.jsonl
    pub last_poll_offset: u64,
    /// Loop mode (Planning or Building)
    pub loop_mode: LoopMode,
    /// Git info (branch, dirty status)
    pub git_info: Option<GitInfo>,
    /// Sandbox enabled
    pub sandbox_enabled: bool,
    /// Dangerous mode enabled
    pub dangerous_mode: bool,
    /// Whether the project selector modal is visible
    pub show_project_selector: bool,
    /// Discovered projects for switching
    pub available_projects: Vec<MaestroProject>,
    /// Selected index in project selector
    pub selected_project_index: usize,
    /// Runtime status for discovered tracks (track_id -> status)
    pub track_runtime_statuses: std::collections::HashMap<String, ConductorStatus>,
    /// Recent iteration logs for the current track
    pub iteration_logs: Vec<leindex_core::orchestrate::model::IterationLog>,
    /// Memories associated with the current track
    pub track_memories: Vec<leindex_core::memory::models::Memory>,
 /// OMP agent status for current track
 pub omp_agent_status: Option<crate::omp::OmpWorkerStatus>,
    /// LSP diagnostic errors from last check
    pub lsp_diagnostics_errors: Vec<String>,
    /// LSP diagnostic warnings from last check
    pub lsp_diagnostics_warnings: Vec<String>,
    /// Whether LSP diagnostics are enabled for this session
    pub lsp_diagnostics_enabled: bool,
    /// Running LSP servers for this session
    pub running_lsp_servers: Vec<String>,
}

impl Default for ConductorState {
    fn default() -> Self {
        Self {
            status: ConductorStatus::Ready,
            session_id: None,
            current_iteration: 0,
            max_iterations: 0,
            current_track: None,
            current_task: None,
            tasks_completed: 0,
            total_tasks: 0,
            started_at: None,
            elapsed_secs: 0,
            current_output: String::new(),
            current_stderr: String::new(),
            subagents: Vec::new(),
            active_agent: None,
            rate_limit: None,
            last_poll_iteration: 0,
            last_poll_offset: 0,
            loop_mode: LoopMode::Building,
            git_info: None,
            sandbox_enabled: false,
            dangerous_mode: false,
            show_project_selector: false,
            available_projects: Vec::new(),
            selected_project_index: 0,
            track_runtime_statuses: std::collections::HashMap::new(),
 omp_agent_status: None,
            iteration_logs: Vec::new(),
            track_memories: Vec::new(),
            lsp_diagnostics_errors: Vec::new(),
            lsp_diagnostics_warnings: Vec::new(),
            lsp_diagnostics_enabled: true,
            running_lsp_servers: Vec::new(),
        }
    }
}

/// Ralph: EngineSubagentState
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentState {
    pub id: String,
    pub agent_type: String,
    pub description: String,
    pub status: SubagentStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub duration_ms: Option<u64>,
    pub depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentStatus {
    #[default]
    Running,
    Completed,
    Error,
}

/// Git repository info for display
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitInfo {
    pub repo_name: Option<String>,
    pub branch: Option<String>,
    pub is_dirty: bool,
    pub commit_hash: Option<String>,
}

/// Ralph: EngineEvent → Maestro: ConductorEvent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConductorEvent {
    // Engine lifecycle
    Started {
        session_id: String,
        total_tasks: usize,
    },
    Stopped {
        reason: StopReason,
        total_iterations: u64,
    },
    Paused,
    Resumed,
    Warning {
        message: String,
    },

    // Iteration lifecycle
    IterationStarted {
        iteration: u64,
        task_id: String,
    },
    IterationCompleted {
        iteration: u64,
        task_completed: bool,
        duration_ms: u64,
    },
    IterationFailed {
        iteration: u64,
        error: String,
    },
    IterationRetrying {
        iteration: u64,
        attempt: u32,
        delay_ms: u64,
    },
    IterationSkipped {
        iteration: u64,
        task_id: String,
        reason: String,
    },
    IterationRateLimited {
        task_id: String,
        retry_attempt: u32,
        delay_ms: u64,
    },

    // Task lifecycle
    TaskSelected {
        task_id: String,
        iteration: u64,
    },
    TaskActivated {
        task_id: String,
    },
    TaskCompleted {
        task_id: String,
        iteration: u64,
    },

    // Agent events
    AgentOutput {
        stream: OutputStream,
        data: String,
    },
    AgentSwitched {
        previous: String,
        new: String,
        reason: AgentReason,
    },
    AllAgentsLimited {
        tried_agents: Vec<String>,
    },
    AgentRecoveryAttempted {
        primary: String,
        fallback: String,
        success: bool,
    },

    // Progress
    AllComplete {
        total_completed: usize,
        total_iterations: u64,
    },
    TasksRefreshed {
        task_count: usize,
    },

    // LSP Diagnostics
    /// LSP diagnostic check started
    DiagnosticsStarted {
        file_count: usize,
    },
    /// LSP diagnostics completed with results
    DiagnosticsCompleted {
        file_count: usize,
        error_count: usize,
        warning_count: usize,
        diagnostics: Vec<String>, // Formatted diagnostic messages
    },
    /// LSP diagnostics failed to run
    DiagnosticsFailed {
        error: String,
    },
    /// LSP status check completed
    LspStatusUpdated {
        lsp_servers: Vec<String>, // Names of running LSPs
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    Completed,
    MaxIterations,
    Interrupted,
    Error,
    NoTasks,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// Ralph: DetailsViewMode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetailsViewMode {
    #[default]
    /// Task metadata, dependencies, description
    Details,
    /// Full-height scrollable iteration output
    Output,
    /// Rendered prompt preview
    Prompt,
}

/// Ralph: IterationTimingInfo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationTiming {
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub is_running: bool,
    pub model: Option<String>,
}

/// A flattened representation of the track/task tree for navigation
#[derive(Debug, Clone)]
pub enum SelectableItem {
    Track {
        index: usize,
        id: String,
        is_master: bool,
        is_external: bool, // Session discovered in ~/.maestro/orchestrate but not in tracks.md
    },
    Task {
        id: String,
        title: String,
        depth: usize,
        status: leindex_core::orchestrate::model::TrackStatus,
        has_children: bool,
        is_expanded: bool,
    },
}
