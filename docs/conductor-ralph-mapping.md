# Conductor: Ralph TUI → Maestro Comprehensive Mapping

This document provides a complete mapping of Ralph TUI components, systems, and concepts to their Maestro Conductor equivalents. It serves as the authoritative blueprint for the full Conductor implementation.

---

## Table of Contents

1. [Philosophy Alignment](#1-philosophy-alignment)
2. [Component Mapping](#2-component-mapping)
3. [Data Model Mapping](#3-data-model-mapping)
4. [Engine & State Machine](#4-engine--state-machine)
5. [UI Layout Specification](#5-ui-layout-specification)
6. [Session & Persistence](#6-session--persistence)
7. [Agent Integration](#7-agent-integration)
8. [LeIndex Integration](#8-leindex-integration)
9. [Implementation Status](#9-implementation-status)
10. [File Structure](#10-file-structure)

---

## 1. Philosophy Alignment

### Ralph Philosophy (from ADR)

| Principle | Description |
|-----------|-------------|
| **Productive Autonomy** | Junior engineer persona - capable execution with structured loop |
| **Minimalist Interface** | High information density, no decorative clutter |
| **Goal-Centric** | Everything revolves around the current task/mission |
| **Observable Loop** | Observe → Think/Plan → Act → Verify cycle |
| **Terminal Transparency** | Real-time visibility into agent actions |
| **Intervention Points** | User can pause at "Think" phase before "Act" |

### Maestro Adaptation

| Ralph Principle | Maestro Equivalent |
|-----------------|-------------------|
| Productive Autonomy | Multi-agent orchestration with LeIndex context |
| Minimalist Interface | Ratatui-based TUI with focused panels |
| Goal-Centric | Track → Task hierarchy from `tracks.md` / `plan.md` |
| Observable Loop | Iteration logs + live output streaming |
| Terminal Transparency | Live output panel with tool highlighting |
| Intervention Points | Pause/Resume controls, per-iteration checkpoints |

---

## 2. Component Mapping

### 2.1 TUI Components

| Ralph Component | File | Maestro Equivalent | Status |
|-----------------|------|-------------------|--------|
| `Header` | `tui/components/Header.tsx` | `conductor/header.rs` | ❌ To Build |
| `Footer` | `tui/components/Footer.tsx` | `conductor/footer.rs` | ❌ To Build |
| `LeftPanel` | `tui/components/LeftPanel.tsx` | `conductor/track_tree.rs` | ⚠️ Partial |
| `RightPanel` | `tui/components/RightPanel.tsx` | `conductor/details_panel.rs` | ⚠️ Partial |
| `ProgressDashboard` | `tui/components/ProgressDashboard.tsx` | `conductor/dashboard.rs` | ❌ To Build |
| `SubagentTreePanel` | `tui/components/SubagentTreePanel.tsx` | `conductor/subagent_tree.rs` | ❌ To Build |
| `TabBar` | `tui/components/TabBar.tsx` | Cockpit tab system (exists) | ✅ Exists |
| `HelpOverlay` | `tui/components/HelpOverlay.tsx` | Cockpit help modal (exists) | ✅ Exists |
| `TaskDetailView` | `tui/components/TaskDetailView.tsx` | `conductor/task_detail.rs` | ❌ To Build |
| `IterationHistoryView` | `tui/components/IterationHistoryView.tsx` | `conductor/iteration_history.rs` | ❌ To Build |

### 2.2 Engine Components

| Ralph Component | File | Maestro Equivalent | Status |
|-----------------|------|-------------------|--------|
| `ExecutionEngine` | `engine/index.ts` | `orchestrate/engine.rs` | ✅ Exists |
| `RateLimitDetector` | `engine/rate-limit-detector.ts` | `orchestrate/rate_limit.rs` | ❌ To Build |
| `AutoCommit` | `engine/auto-commit.ts` | `orchestrate/commit.rs` | ❌ To Build |

### 2.3 Session Components

| Ralph Component | File | Maestro Equivalent | Status |
|-----------------|------|-------------------|--------|
| Session Lock | `session/lock.ts` | `orchestrate/state.rs` (LockGuard) | ✅ Exists |
| Session Persistence | `session/persistence.ts` | `orchestrate/state.rs` (StateManager) | ✅ Exists |
| Session Registry | `session/registry.ts` | N/A (Maestro uses file-based) | N/A |

### 2.4 Plugin System

| Ralph Component | Maestro Equivalent | Notes |
|-----------------|-------------------|-------|
| Agent Plugins (`claude`, `opencode`, etc.) | `orchestrate/runner.rs` (CliRunner) | Maestro supports: claude, gemini, qwen, opencode, amp, codex, droid, maestro |
| Tracker Plugins (`beads`, `json`) | `orchestrate/parser.rs` | Maestro uses `tracks.md` + `plan.md` format |
| Template System | `orchestrate/prompts.rs` | LeIndex-powered context bundles |

---

## 3. Data Model Mapping

### 3.1 Task/Track Models

```
┌─────────────────────────────────────────────────────────────────┐
│                    RALPH                    MAESTRO             │
├─────────────────────────────────────────────────────────────────┤
│ TrackerTask                          Track                      │
│ ├─ id: string                        ├─ id: String              │
│ ├─ title: string                     ├─ description: String     │
│ ├─ status: TrackerTaskStatus         ├─ status: TrackStatus     │
│ ├─ description?: string              ├─ link_path: PathBuf      │
│ ├─ priority?: TaskPriority           ├─ metadata: TrackMetadata │
│ ├─ dependsOn?: string[]              └─ plan: TrackPlan         │
│ ├─ blocks?: string[]                                            │
│ ├─ parentId?: string                 Task                       │
│ └─ metadata?: Record<...>            ├─ id: String              │
│                                      ├─ title: String           │
│ TaskStatus (TUI display)             ├─ status: TrackStatus     │
│ ├─ done                              ├─ dependencies: Vec<...>  │
│ ├─ active                            ├─ description: String     │
│ ├─ actionable                        ├─ subtasks: Vec<Task>     │
│ ├─ pending                           ├─ notes: Option<String>   │
│ ├─ blocked                           └─ line_number: usize      │
│ ├─ error                                                        │
│ └─ closed                            TrackStatus                │
│                                      ├─ Pending    ([ ])        │
│                                      ├─ InProgress ([~])        │
│                                      └─ Completed  ([x])        │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Engine State Models

```rust
// Maestro Conductor State (to implement in conductor/model.rs)

/// Ralph: RalphStatus → Maestro: ConductorStatus
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConductorStatus {
    Ready,      // Ralph: ready - waiting for user to start
    Running,    // Ralph: running - generic running state
    Selecting,  // Ralph: selecting - selecting next task
    Executing,  // Ralph: executing - agent running on task
    Pausing,    // Ralph: pausing - pause requested, finishing iteration
    Paused,     // Ralph: paused - waiting to resume
    Stopping,   // Ralph: stopped - shutting down
    Completed,  // Ralph: complete - all tasks finished
    Idle,       // Ralph: idle - no more tasks available
    Failed,     // Ralph: error - stopped due to error
}

/// Ralph: ActiveAgentState
#[derive(Debug, Clone)]
pub struct ActiveAgentState {
    /// Agent tool name (claude, gemini, qwen, opencode, etc.)
    pub tool: String,
    /// Model identifier (optional)
    pub model: Option<String>,
    /// Why this agent is active
    pub reason: AgentReason,
    /// When this agent became active
    pub since: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentReason {
    Primary,   // Configured primary agent
    Fallback,  // Switched due to rate limit or error
}

/// Ralph: RateLimitState
#[derive(Debug, Clone)]
pub struct RateLimitState {
    /// Primary agent that was rate limited
    pub primary_agent: String,
    /// When the primary was rate limited (None if not limited)
    pub limited_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Current fallback agent in use (None if using primary)
    pub fallback_agent: Option<String>,
    /// Current retry count for rate limit backoff
    pub retry_count: u32,
    /// When backoff expires
    pub backoff_until: Option<std::time::Instant>,
    /// Last rate limit message from agent
    pub last_message: Option<String>,
}

/// Ralph: EngineState → Maestro: ConductorState
#[derive(Debug, Clone)]
pub struct ConductorState {
    /// Current execution status
    pub status: ConductorStatus,
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
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
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
    /// Loop mode
    pub loop_mode: LoopMode,
    /// Git info (branch, dirty status)
    pub git_info: Option<GitInfo>,
    /// Sandbox enabled
    pub sandbox_enabled: bool,
    /// Dangerous mode enabled
    pub dangerous_mode: bool,
}

/// Ralph: EngineSubagentState
#[derive(Debug, Clone)]
pub struct SubagentState {
    pub id: String,
    pub agent_type: String,
    pub description: String,
    pub status: SubagentStatus,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub duration_ms: Option<u64>,
    pub depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubagentStatus {
    Running,
    Completed,
    Error,
}

/// Git repository info for display
#[derive(Debug, Clone)]
pub struct GitInfo {
    pub repo_name: Option<String>,
    pub branch: Option<String>,
    pub is_dirty: bool,
    pub commit_hash: Option<String>,
}
```

### 3.3 Event System

```rust
/// Ralph: EngineEvent → Maestro: ConductorEvent
#[derive(Debug, Clone)]
pub enum ConductorEvent {
    // Engine lifecycle
    Started { session_id: String, total_tasks: usize },
    Stopped { reason: StopReason, total_iterations: u64 },
    Paused,
    Resumed,
    Warning { message: String },
    
    // Iteration lifecycle
    IterationStarted { iteration: u64, task_id: String },
    IterationCompleted { iteration: u64, task_completed: bool, duration_ms: u64 },
    IterationFailed { iteration: u64, error: String },
    IterationRetrying { iteration: u64, attempt: u32, delay_ms: u64 },
    IterationSkipped { iteration: u64, task_id: String, reason: String },
    IterationRateLimited { task_id: String, retry_attempt: u32, delay_ms: u64 },
    
    // Task lifecycle
    TaskSelected { task_id: String, iteration: u64 },
    TaskActivated { task_id: String },
    TaskCompleted { task_id: String, iteration: u64 },
    
    // Agent events
    AgentOutput { stream: OutputStream, data: String },
    AgentSwitched { previous: String, new: String, reason: AgentReason },
    AllAgentsLimited { tried_agents: Vec<String> },
    AgentRecoveryAttempted { primary: String, fallback: String, success: bool },
    
    // Progress
    AllComplete { total_completed: usize, total_iterations: u64 },
    TasksRefreshed { task_count: usize },
}

#[derive(Debug, Clone, Copy)]
pub enum StopReason {
    Completed,
    MaxIterations,
    Interrupted,
    Error,
    NoTasks,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputStream {
    Stdout,
    Stderr,
}
```

### 3.4 View Mode

```rust
/// Ralph: DetailsViewMode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailsViewMode {
    #[default]
    Details,  // Task metadata, dependencies, description
    Output,   // Full-height scrollable iteration output
    Prompt,   // Rendered prompt preview
}

/// Ralph: IterationTimingInfo
#[derive(Debug, Clone)]
pub struct IterationTiming {
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<u64>,
    pub is_running: bool,
    pub model: Option<String>,
}
```

---

## 4. Engine & State Machine

### 4.1 Ralph Loop State Machine

```
                    ┌──────────────────────────────────────────┐
                    │                                          │
                    ▼                                          │
              ┌─────────┐                                      │
    ┌────────▶│  Ready  │◀────────────────────────┐            │
    │         └────┬────┘                         │            │
    │              │ [s] Start                    │            │
    │              ▼                              │            │
    │         ┌─────────┐                         │            │
    │         │ Running │─────────────────────────┤            │
    │         └────┬────┘                         │            │
    │              │                              │            │
    │              ▼                              │            │
    │         ┌──────────┐     ┌──────────┐       │            │
    │         │Selecting │────▶│Executing │       │            │
    │         └────┬─────┘     └────┬─────┘       │            │
    │              │                │             │            │
    │              │                │ [p] Pause   │            │
    │              │                ▼             │            │
    │              │          ┌─────────┐         │            │
    │              │          │ Pausing │─────────┤            │
    │              │          └────┬────┘         │            │
    │              │               │              │            │
    │              │               ▼              │            │
    │              │          ┌─────────┐         │            │
    │              │          │ Paused  │         │            │
    │              │          └────┬────┘         │            │
    │              │               │ [r] Resume   │            │
    │              │               ▼              │            │
    │              │          ┌─────────┐         │            │
    │              └─────────▶│ Running │─────────┘            │
    │                         └────┬────┘                      │
    │                              │                           │
    │ [q] Quit                     │ All tasks complete        │
    │                              ▼                           │
    │                        ┌──────────┐                      │
    └────────────────────────│ Complete │──────────────────────┘
                             └──────────┘
                                   │
                                   ▼
                             ┌──────────┐
                             │  Idle    │ (no more tasks)
                             └──────────┘
                                   │
                                   ▼
                             ┌──────────┐
                             │  Failed  │ (on error)
                             └──────────┘
```

### 4.2 Maestro Conductor State Machine (Rust)

```rust
// In conductor/state_machine.rs

impl ConductorState {
    pub fn can_start(&self) -> bool {
        matches!(self.status, ConductorStatus::Ready | ConductorStatus::Idle)
    }
    
    pub fn can_pause(&self) -> bool {
        matches!(self.status, ConductorStatus::Running | ConductorStatus::Executing | ConductorStatus::Selecting)
    }
    
    pub fn can_resume(&self) -> bool {
        matches!(self.status, ConductorStatus::Paused)
    }
    
    pub fn transition(&mut self, event: ConductorEvent) {
        match (&self.status, &event) {
            (ConductorStatus::Ready, ConductorEvent::Started { .. }) => {
                self.status = ConductorStatus::Running;
            }
            (ConductorStatus::Running, ConductorEvent::TaskSelected { .. }) => {
                self.status = ConductorStatus::Selecting;
            }
            (ConductorStatus::Selecting, ConductorEvent::IterationStarted { .. }) => {
                self.status = ConductorStatus::Executing;
            }
            (ConductorStatus::Executing, ConductorEvent::IterationCompleted { .. }) => {
                self.status = ConductorStatus::Running;
            }
            (_, ConductorEvent::Paused) => {
                self.status = ConductorStatus::Paused;
            }
            (ConductorStatus::Paused, ConductorEvent::Resumed) => {
                self.status = ConductorStatus::Running;
            }
            (_, ConductorEvent::AllComplete { .. }) => {
                self.status = ConductorStatus::Completed;
            }
            (_, ConductorEvent::Stopped { reason: StopReason::Error, .. }) => {
                self.status = ConductorStatus::Failed;
            }
            _ => {}
        }
    }
}
```

### 4.3 Rate Limit Detection

```rust
// In conductor/rate_limit.rs

/// Patterns to detect rate limiting in agent output
const RATE_LIMIT_PATTERNS: &[&str] = &[
    "rate limit",
    "rate_limit",
    "ratelimit",
    "too many requests",
    "429",
    "quota exceeded",
    "capacity",
    "overloaded",
    "try again later",
    "retry after",
];

pub struct RateLimitDetector {
    patterns: Vec<regex::Regex>,
}

impl RateLimitDetector {
    pub fn detect(&self, output: &str) -> Option<RateLimitDetection> {
        let lower = output.to_lowercase();
        for pattern in RATE_LIMIT_PATTERNS {
            if lower.contains(pattern) {
                return Some(RateLimitDetection {
                    detected: true,
                    message: self.extract_message(output),
                    retry_after: self.extract_retry_after(output),
                });
            }
        }
        None
    }
    
    fn extract_retry_after(&self, output: &str) -> Option<u64> {
        // Parse "retry after X seconds" or similar
        // ...
    }
}
```

---

## 5. UI Layout Specification

### 5.1 Full Layout Structure

```
┌────────────────────────────────────────────────────────────────────────────┐
│ Header (1-2 lines)                                                         │
│ ● Running → auth-fix/task-3.2 │ claude (sonnet) │ beads │ 🔒 auto │ ▓▓▓░░ 3/5 [7/∞] ⏱ 5m 23s │
├───────────────────────────────┬────────────────────────────────────────────┤
│ Left Panel (35-40%)           │ Right Panel (60-65%)                       │
│                               │ ┌────────────────────────────────────────┐ │
│ ┌───────────────────────────┐ │ │ Progress Dashboard (optional, 8 lines) │ │
│ │ Tasks [Details]           │ │ │ Status: Running | Track: auth-fix      │ │
│ │                           │ │ │ Agent: claude | Model: sonnet          │ │
│ │ ✓ 1 Setup environment     │ │ │ Tracker: beads | Git: main*            │ │
│ │ ▶ 2 Implement auth flow   │ │ │ Sandbox: 🔒 auto | Commit: ✓ auto      │ │
│ │   ○ 2.1 Create middleware │ │ └────────────────────────────────────────┘ │
│ │   ○ 2.2 Add JWT validation│ │ ┌────────────────────────────────────────┐ │
│ │ ○ 3 Write tests           │ │ │ Details [Details|Output|Prompt]        │ │
│ │ ⊘ 4 Deploy (blocked by 3) │ │ │                                        │ │
│ │                           │ │ │ Task: 2.1 Create auth middleware       │ │
│ │                           │ │ │ Status: ○ Pending (actionable)         │ │
│ │                           │ │ │ Dependencies: 1 (completed)            │ │
│ │                           │ │ │                                        │ │
│ │                           │ │ │ Description:                           │ │
│ │                           │ │ │ Create Express middleware that...      │ │
│ │                           │ │ │                                        │ │
│ │                           │ │ │ Acceptance Criteria:                   │ │
│ │                           │ │ │ [ ] Validates JWT tokens               │ │
│ │                           │ │ │ [ ] Returns 401 on invalid             │ │
│ │                           │ │ │ [ ] Attaches user to request           │ │
│ └───────────────────────────┘ │ └────────────────────────────────────────┘ │
│                               │ ┌────────────────────────────────────────┐ │
│ ┌───────────────────────────┐ │ │ Agent Tree (optional)                  │ │
│ │ Agent Tree (2 active)     │ │ │ ◉ task-2.1 Create auth middleware (2) │ │
│ │ ◉ task-2.1 (2)            │ │ │   ◐ [Read] Analyzing src/auth/...     │ │
│ │   ◐ [Read] Analyzing...   │ │ │   ✓ [Bash] npm install jsonwebtoken   │ │
│ │   ✓ [Bash] npm install... │ │ └────────────────────────────────────────┘ │
│ └───────────────────────────┘ │                                            │
├───────────────────────────────┴────────────────────────────────────────────┤
│ Footer (1-3 lines)                                                         │
│ q:Quit  s:Start  p:Pause  r:Refresh  o:Cycle Views  d:Dashboard  ?:Help    │
└────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Component Specifications

#### Header Component

```rust
// conductor/header.rs

pub struct HeaderProps {
    pub status: ConductorStatus,
    pub elapsed_secs: u64,
    pub current_task_id: Option<String>,
    pub current_task_title: Option<String>,
    pub completed_tasks: usize,
    pub total_tasks: usize,
    pub agent_name: Option<String>,
    pub tracker_name: String,  // "tracks.md" for Maestro
    pub active_agent: Option<ActiveAgentState>,
    pub rate_limit: Option<RateLimitState>,
    pub current_iteration: u64,
    pub max_iterations: u64,
    pub current_model: Option<String>,
    pub sandbox_enabled: bool,
    pub git_info: Option<GitInfo>,
}

pub fn render_header(frame: &mut Frame, area: Rect, props: &HeaderProps, theme: &Theme) {
    // Single line compact header:
    // [status_icon] [status_label] → [task_title] │ [agent] │ [model] │ [tracker] │ [sandbox] │ [progress_bar] X/Y [iter] ⏱ time
}
```

#### Left Panel (Task Tree)

```rust
// conductor/track_tree.rs

pub struct TrackTreeProps {
    pub tracks: Vec<Track>,
    pub selected_track_idx: usize,
    pub selected_task_id: Option<String>,
    pub expanded_tracks: HashSet<String>,
    pub expanded_tasks: HashSet<String>,
    pub is_focused: bool,
}

/// Task display item (flattened for rendering)
pub struct TaskDisplayItem {
    pub id: String,
    pub title: String,
    pub status: TrackStatus,
    pub depth: usize,          // Indentation level
    pub is_track: bool,        // Track header vs task
    pub is_expanded: bool,     // For expandable items
    pub has_children: bool,    // Has subtasks
    pub is_blocked: bool,      // Dependencies not met
    pub is_actionable: bool,   // Ready to work on
}

pub fn render_track_tree(frame: &mut Frame, area: Rect, props: &TrackTreeProps, theme: &Theme) {
    // Render hierarchical tree with status indicators
    // ✓ = completed, ▶ = active, ○ = pending/actionable, ⊘ = blocked, ✗ = error
}
```

#### Right Panel (Details/Output)

```rust
// conductor/details_panel.rs

pub struct DetailsPanelProps {
    pub selected_task: Option<Task>,
    pub current_iteration: u64,
    pub iteration_output: String,
    pub view_mode: DetailsViewMode,
    pub iteration_timing: Option<IterationTiming>,
    pub agent_name: Option<String>,
    pub current_model: Option<String>,
    pub prompt_preview: Option<String>,
    pub is_focused: bool,
}

pub fn render_details_panel(frame: &mut Frame, area: Rect, props: &DetailsPanelProps, theme: &Theme) {
    match props.view_mode {
        DetailsViewMode::Details => render_task_details(frame, area, props, theme),
        DetailsViewMode::Output => render_iteration_output(frame, area, props, theme),
        DetailsViewMode::Prompt => render_prompt_preview(frame, area, props, theme),
    }
}
```

#### Progress Dashboard

```rust
// conductor/dashboard.rs

pub struct DashboardProps {
    pub status: ConductorStatus,
    pub agent_name: String,
    pub current_model: Option<String>,
    pub tracker_name: String,
    pub current_track: Option<String>,
    pub current_task_id: Option<String>,
    pub current_task_title: Option<String>,
    pub sandbox_enabled: bool,
    pub dangerous_mode: bool,
    pub auto_commit: bool,
    pub git_info: Option<GitInfo>,
}

pub fn render_dashboard(frame: &mut Frame, area: Rect, props: &DashboardProps, theme: &Theme) {
    // 8-line dashboard with:
    // Row 1: Status line with indicator
    // Row 2: Track / Task info
    // Row 3: Agent + Model
    // Row 4: Tracker
    // Row 5: Git info (branch, dirty)
    // Row 6: Sandbox + Auto-commit
}
```

#### Subagent Tree Panel

```rust
// conductor/subagent_tree.rs

pub struct SubagentTreeProps {
    pub tree: Vec<SubagentTreeNode>,
    pub active_subagent_id: Option<String>,
    pub current_task_id: Option<String>,
    pub current_task_title: Option<String>,
    pub current_task_status: SubagentStatus,
    pub selected_id: Option<String>,
    pub is_focused: bool,
}

pub struct SubagentTreeNode {
    pub state: SubagentState,
    pub children: Vec<SubagentTreeNode>,
}

pub fn render_subagent_tree(frame: &mut Frame, area: Rect, props: &SubagentTreeProps, theme: &Theme) {
    // ◉ task-2.1 Create auth middleware (2)
    //   ◐ [Read] Analyzing src/auth/...
    //   ✓ [Bash] npm install jsonwebtoken [1.2s]
}
```

### 5.3 Theme Colors (Ralph → Maestro)

```rust
// conductor/theme.rs

pub struct ConductorTheme {
    // Background
    pub bg_primary: Color,      // #1a1b26
    pub bg_secondary: Color,    // #24283b
    pub bg_tertiary: Color,     // #2f3449
    pub bg_highlight: Color,    // #3d4259
    
    // Foreground
    pub fg_primary: Color,      // #c0caf5
    pub fg_secondary: Color,    // #a9b1d6
    pub fg_muted: Color,        // #565f89
    pub fg_dim: Color,          // #414868
    
    // Status
    pub status_success: Color,  // #9ece6a (green)
    pub status_warning: Color,  // #e0af68 (yellow)
    pub status_error: Color,    // #f7768e (red)
    pub status_info: Color,     // #7aa2f7 (blue)
    
    // Task status
    pub task_done: Color,       // #9ece6a
    pub task_active: Color,     // #9ece6a
    pub task_actionable: Color, // #9ece6a
    pub task_pending: Color,    // #565f89
    pub task_blocked: Color,    // #f7768e
    pub task_error: Color,      // #f7768e
    pub task_closed: Color,     // #414868
    
    // Accent
    pub accent_primary: Color,   // #7aa2f7
    pub accent_secondary: Color, // #bb9af7
    pub accent_tertiary: Color,  // #7dcfff
}

// Status indicators (Unicode)
pub const STATUS_DONE: &str = "✓";
pub const STATUS_ACTIVE: &str = "▶";
pub const STATUS_ACTIONABLE: &str = "○";
pub const STATUS_PENDING: &str = "○";
pub const STATUS_BLOCKED: &str = "⊘";
pub const STATUS_ERROR: &str = "✗";
pub const STATUS_RUNNING: &str = "◐";
pub const STATUS_PAUSED: &str = "⏸";
pub const STATUS_READY: &str = "◉";
```

---

## 6. Session & Persistence

### 6.1 Maestro Session State (Enhanced)

```rust
// In orchestrate/state.rs (extend existing)

/// Enhanced session state with Ralph-style crash recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSessionState {
    // Existing fields from SessionState
    pub track_id: String,
    pub mode: LoopMode,
    pub agent_config: AgentConfig,
    pub current_iteration: u64,
    pub current_task_id: Option<String>,
    pub started_at: String,
    pub updated_at: String,
    pub status: SessionStatus,
    
    // New fields for Ralph parity
    /// Maximum iterations (0 = unlimited)
    pub max_iterations: u64,
    /// Tasks completed this session
    pub tasks_completed: usize,
    /// Whether session is paused
    pub is_paused: bool,
    /// When session was paused
    pub paused_at: Option<String>,
    /// Task IDs actively being worked on (for crash recovery)
    pub active_task_ids: Vec<String>,
    /// Skipped task IDs (for retry/skip error handling)
    pub skipped_task_ids: Vec<String>,
    /// Subagent panel visibility preference
    pub subagent_panel_visible: bool,
    /// View mode preference
    pub details_view_mode: String, // "details" | "output" | "prompt"
}
```

### 6.2 Crash Recovery

```rust
// In orchestrate/state.rs

impl StateManager {
    /// Detect and recover from stale session (Ralph pattern)
    pub fn detect_and_recover_stale(&self, track_id: &str) -> Result<StaleRecoveryResult> {
        let lock_path = self.lock_file_path(track_id);
        let session = self.load_session(track_id)?;
        
        if let Some(session) = session {
            // If session was "Running" but lock is stale, it crashed
            if session.status == SessionStatus::Running {
                if !Self::is_lock_valid_file(&lock_path)? {
                    // Recover: clear active tasks, set to Interrupted
                    let mut recovered = session;
                    recovered.status = SessionStatus::Interrupted;
                    recovered.active_task_ids.clear();
                    recovered.updated_at = Utc::now().to_rfc3339();
                    self.save_session(&recovered)?;
                    
                    return Ok(StaleRecoveryResult {
                        was_stale: true,
                        cleared_task_count: session.active_task_ids.len(),
                        previous_status: SessionStatus::Running,
                    });
                }
            }
        }
        
        Ok(StaleRecoveryResult::default())
    }
}
```

---

## 7. Agent Integration

### 7.1 Agent Runner Mapping

| Ralph Agent Plugin | Maestro Runner | CLI Command |
|--------------------|----------------|-------------|
| `claude` | `CliRunner` | `claude --dangerously-skip-permissions` |
| `opencode` | `CliRunner` | `opencode --non-interactive` |
| `codex` | `CliRunner` | `codex --quiet` |
| `gemini` | `CliRunner` | `gemini` |
| `qwen` | `CliRunner` | `qwen` |
| `amp` | `CliRunner` | `amp --print` |
| `droid` | `CliRunner` | `droid` |

### 7.2 Fallback Chain Configuration

```rust
// In orchestrate/model.rs (extend AgentConfig)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub tool: String,
    pub model: Option<String>,
    pub dangerous_mode: bool,
    pub sandbox: bool,
    /// Fallback agents when primary is rate-limited
    pub fallback_chain: Option<Vec<FallbackAgent>>,
    /// Rate limit handling configuration
    pub rate_limit_handling: Option<RateLimitConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackAgent {
    pub tool: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub max_retries: u32,
    pub backoff_base_secs: u64,
    pub backoff_max_secs: u64,
    pub use_fallback: bool,
}
```

---

## 8. LeIndex Integration

### 8.1 Context Bundle Generation

```rust
// In orchestrate/context.rs

/// Generate LeIndex context bundle for an iteration
pub async fn build_iteration_context(
    track_id: &str,
    task: &Task,
    budget_tokens: usize,
    mode: FormatMode,
) -> Result<ContextBundle> {
    // 1. Run phase1 (structural scan)
    let phase1 = leindex_phase1(&task.related_files)?;
    
    // 2. Run phase2 (dependency map) 
    let phase2 = leindex_phase2(&task.related_files)?;
    
    // 3. Build task context
    let task_context = format!(
        "## Current Task\n\n**ID**: {}\n**Title**: {}\n**Description**: {}\n",
        task.id, task.title, task.description
    );
    
    // 4. Get recent iteration summaries
    let recent = get_recent_iterations(track_id, 5)?;
    let recent_summary = format_iteration_summary(&recent);
    
    // 5. Assemble bundle within budget
    let bundle = ContextBundle::new()
        .add_section("task", task_context)
        .add_section("structure", phase1)
        .add_section("dependencies", phase2)
        .add_section("recent_progress", recent_summary)
        .fit_to_budget(budget_tokens, mode)?;
    
    Ok(bundle)
}

#[derive(Debug, Clone)]
pub struct ContextBundle {
    pub preamble_text: String,
    pub sources: Vec<ContextSource>,
    pub token_count: usize,
}

#[derive(Debug, Clone)]
pub struct ContextSource {
    pub source_type: String,  // "leindex", "task", "history"
    pub path: Option<String>,
    pub token_count: usize,
}
```

---

## 9. Implementation Status

### 9.1 Completed ✅

| Component | File | Notes |
|-----------|------|-------|
| Path resolution | `maestro_paths.rs` | Smart project discovery |
| Basic conductor pane | `conductor.rs` | Renamed from orchestrate |
| Track/Task models | `model.rs` | Full data models |
| Parser | `parser.rs` | tracks.md + plan.md |
| Session state | `state.rs` | Lock + persistence |
| Basic engine | `engine.rs` | Iteration loop |
| CLI runner | `runner.rs` | Multi-agent support |

### 9.2 Partial ⚠️

| Component | Current State | Remaining Work |
|-----------|---------------|----------------|
| Track tree UI | Basic list | Hierarchical display, status colors, expand/collapse |
| Details panel | Basic info | View mode toggle, output streaming, prompt preview |
| Keybindings | Basic navigation | Full Ralph keybindings |

### 9.3 To Build ❌

| Component | Priority | Effort |
|-----------|----------|--------|
| Header component | High | S |
| Footer component | High | S |
| Progress dashboard | Medium | M |
| ConductorState model | High | M |
| Event system | High | M |
| Live polling | High | M |
| Rate limit detection | Medium | M |
| Subagent tree | Low | L |
| View mode toggle | Medium | S |
| Iteration timing | Medium | S |
| Git info display | Low | S |
| Setup wizard | Low | L |

---

## 10. File Structure

### 10.1 Target Structure

```
crates/cockpit/src/
├── conductor/
│   ├── mod.rs              # Module exports
│   ├── model.rs            # ConductorState, events, types
│   ├── state_machine.rs    # State transitions
│   ├── header.rs           # Header component
│   ├── footer.rs           # Footer component  
│   ├── track_tree.rs       # Left panel (task tree)
│   ├── details_panel.rs    # Right panel (details/output/prompt)
│   ├── dashboard.rs        # Progress dashboard
│   ├── subagent_tree.rs    # Subagent hierarchy panel
│   ├── iteration_history.rs # Iteration history view
│   ├── theme.rs            # Conductor-specific colors/indicators
│   ├── keybindings.rs      # Key handling
│   └── polling.rs          # Live state polling
├── conductor.rs            # Main conductor pane (existing)
├── maestro_paths.rs        # Path resolution (existing)
└── ...

maestro/leindex/rust/src/orchestrate/
├── mod.rs
├── model.rs                # Track, Task, Session models
├── engine.rs               # Execution loop
├── runner.rs               # Agent execution
├── parser.rs               # tracks.md / plan.md parsing
├── prompts.rs              # Prompt building
├── state.rs                # Session persistence
├── setup.rs                # First-run setup
├── rate_limit.rs           # Rate limit detection (NEW)
├── context.rs              # LeIndex context building (NEW)
└── commit.rs               # Auto-commit logic (NEW)
```

---

## Summary

This mapping provides the comprehensive blueprint for porting Ralph TUI functionality into Maestro's Conductor. The key insight is that **Maestro already has the core engine infrastructure** (engine.rs, runner.rs, state.rs, parser.rs) - what's missing is:

1. **Rich UI components** (Header, Footer, Dashboard, Subagent Tree)
2. **State machine with events** (ConductorState, ConductorEvent)
3. **Live polling and updates** (session.json monitoring)
4. **Rate limit detection and fallback chains**
5. **View mode toggle** (Details ↔ Output ↔ Prompt)
6. **LeIndex context injection** per iteration

The existing Maestro code is well-designed and should be extended, not replaced.
