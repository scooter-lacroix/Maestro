# Conductor Implementation Plan — Target 100% Ralph-TUI Parity

## Overview

This plan closes all 57 gaps identified in the gap analysis to bring the Conductor to 100% feature parity with Ralph-TUI, plus Maestro-specific deep integration with CLI tools, memory system, and steering.

**Scope:** Both Conductor pane (`crates/cockpit/src/conductor/`) AND the engine control surface (`maestro/leindex/rust/src/orchestrate/control.rs`, `engine.rs`) where the Conductor needs new control commands to function.

**Total: 7 phases, 25-30 working days.**

---

## Phase 1: UI Primitives + Steering (4-5 days)

**Goal:** Build the two missing modal primitives that unblock everything else, then wire up steering — the single most requested feature.

### 1.1 Create `input_modal.rs` — Text Input Modal

**New file:** `crates/cockpit/src/conductor/input_modal.rs`

A reusable text input overlay for the TUI. Follows the `centered_rect` + `Clear` pattern from `project_selector.rs`.

```rust
pub struct InputModal {
    pub title: String,
    pub prompt_text: String,
    pub input_buffer: String,
    pub cursor_pos: usize,
    pub visible: bool,
    pub multiline: bool,
}

impl InputModal {
    pub fn new(title: &str, prompt: &str) -> Self { /* ... */ }
    pub fn handle_key(&mut self, key: KeyEvent) -> InputAction { /* ... */ }
    pub fn take_input(&mut self) -> String { /* ... */ }
}

pub enum InputAction {
    None,
    Submit(String),
    Cancel,
    Handled,
}

pub fn render_input_modal(f: &mut Frame, area: Rect, modal: &InputModal, theme: &Theme) {
    if !modal.visible { return; }
    let panel = centered_rect(60, 30, area);
    f.render_widget(Clear, panel);
    // Block with title, prompt text, input field with cursor, footer hints
}
```

Supports: character input, backspace, cursor movement, Enter to submit, Esc to cancel, optional multiline.

### 1.2 Create `selector_modal.rs` — List Selector Modal

**New file:** `crates/cockpit/src/conductor/selector_modal.rs`

```rust
pub struct SelectorModal {
    pub title: String,
    pub items: Vec<SelectorItem>,
    pub selected: usize,
    pub visible: bool,
}

pub struct SelectorItem {
    pub label: String,
    pub value: String,
    pub description: Option<String>,
}

pub enum SelectorAction {
    None,
    Selected(String),
    Cancel,
}
```

### 1.3 Add Steering — Engine + Conductor

**Engine-side (`orchestrate/control.rs`):** Add new `ControlCommand` variant:

```rust
// Add to ControlCommand enum:
Steer { message: String },
```

**Engine-side (`orchestrate/engine.rs`):** Read steering in the main loop:

```rust
// In the control command check section:
ControlCommand::Steer { message } => {
    info!("Steering message from conductor: {}", message);
    // Store for next prompt injection
    self.pending_steering = Some(message);
}
```

**Engine-side (`orchestrate/engine.rs`):** Inject in `build_iteration_prompt()`:

```rust
// After memory_context section:
if let Some(ref steering) = self.pending_steering {
    prompt.push_str("\n## User Steering Message\n\n");
    prompt.push_str(&format!("> {}\n\n", steering));
    prompt.push_str("**Follow this guidance for this iteration.**\n\n");
}
```

Clear `pending_steering` after injection (single-use).

**Conductor-side (`keybindings.rs`):**

```rust
// New keybinding: Ctrl+M for steering message
(KeyModifiers::CONTROL, KeyCode::Char('m')) => {
    pane.steering_modal.title = "Steering Message".to_string();
    pane.steering_modal.prompt_text = "Enter guidance for the next iteration:".to_string();
    pane.steering_modal.visible = true;
    ConductorAction::Handled
}
```

**Conductor-side (`pane.rs`):** Add modal fields:

```rust
pub struct ConductorPane {
    // ... existing fields ...
    pub steering_modal: InputModal,
    pub selector_modal: SelectorModal,
}
```

**Conductor-side (`pane.rs` render):** Render modal overlay:

```rust
// After conflict panel / project selector renders:
if pane.steering_modal.visible {
    crate::conductor::input_modal::render_input_modal(frame, area, &pane.steering_modal, theme);
}
```

**Conductor-side (`keybindings.rs`):** Handle modal input when visible:

```rust
// At the TOP of handle_key_event, before all other handlers:
if pane.steering_modal.visible {
    match pane.steering_modal.handle_key(key) {
        InputAction::Submit(text) => {
            if let Some(track_id) = pane.state.current_track.clone() {
                send_control_command(&track_id, ControlCommandType::Steer { message: text });
            }
            return ConductorAction::StatusMessage("Steering message sent".to_string());
        }
        InputAction::Cancel => return ConductorAction::Handled,
        InputAction::Handled => return ConductorAction::Handled,
        InputAction::None => return ConductorAction::Handled,
    }
}
```

### 1.4 Add remaining `ControlCommand` variants

**Engine-side (`orchestrate/control.rs`):**

```rust
// Add to ControlCommand enum:
SetMaxIterations { max: u64 },
SetMode { mode: String },
SwitchAgent { tool: String, model: Option<String> },
ToggleSandbox,
ToggleDangerous,
```

**Engine-side (`orchestrate/engine.rs`):** Handle in control command loop:

```rust
ControlCommand::SetMaxIterations { max } => {
    session.max_iterations = max;
    self.state_manager.save_session(&session)?;
    info!("Max iterations set to {}", max);
}
ControlCommand::SetMode { mode } => {
    session.mode = match mode.as_str() {
        "planning" => LoopMode::Planning,
        _ => LoopMode::Building,
    };
    self.state_manager.save_session(&session)?;
}
ControlCommand::SwitchAgent { tool, model } => {
    session.agent_config.tool = tool;
    session.agent_config.model = model;
    self.state_manager.save_session(&session)?;
}
ControlCommand::ToggleSandbox => {
    session.agent_config.sandbox = !session.agent_config.sandbox;
    self.state_manager.save_session(&session)?;
}
ControlCommand::ToggleDangerous => {
    session.agent_config.dangerous_mode = !session.agent_config.dangerous_mode;
    self.state_manager.save_session(&session)?;
}
```

### 1.5 Conductor keybindings for runtime config

```rust
// Error strategy selector
(KeyModifiers::NONE, KeyCode::Char('e')) if pane.state.status != ConductorStatus::Ready => {
    pane.selector_modal = SelectorModal::new("Error Strategy", vec![
        ("Retry", "retry"), ("Skip", "skip"), ("Abort", "abort"),
    ]);
    pane.selector_modal.visible = true;
    ConductorAction::Handled
}

// Max iterations (input modal)
(KeyModifiers::NONE, KeyCode::Char('i')) if pane.state.status != ConductorStatus::Ready => {
    pane.iter_modal = InputModal::new("Max Iterations", "Enter max iterations (0 = unlimited):");
    pane.iter_modal.visible = true;
    ConductorAction::Handled
}

// Toggle mode
(KeyModifiers::NONE, KeyCode::Char('m')) => {
    // Toggle between Planning and Building
    let new_mode = if pane.state.loop_mode == LoopMode::Building { "planning" } else { "building" };
    if let Some(track_id) = pane.state.current_track.clone() {
        send_control_command(&track_id, ControlCommandType::SetMode { mode: new_mode.to_string() });
    }
    ConductorAction::StatusMessage(format!("Mode switched to {}", new_mode))
}
```

---

## Phase 2: Task Display & Details (2-3 days)

**Goal:** Close C8-C12 and D3. Pure Conductor-side, zero engine changes.

### 2.1 Wire blocked/actionable indicators in `track_tree.rs`

Extend `SelectableItem::Task` with `is_blocked: bool` and `is_actionable_next: bool`.

In `pane.rs::get_selectable_items()`, call `task.is_blocked(&completed_map)` using `TrackPlan::completed_tasks_map()`.

In `track_tree.rs`, use:

```rust
TrackStatus::Pending => {
    if is_blocked {
        (STATUS_BLOCKED, conductor_theme.task_blocked)       // ⊘ red
    } else {
        (STATUS_ACTIONABLE, conductor_theme.task_actionable) // ○ green
    }
}
```

### 2.2 Enrich Task details view in `details_panel.rs`

When `SelectableItem::Task` is selected, show:
- **Description:** `task.description` (currently not rendered)
- **Dependencies:** list with ✓/○ resolved indicators and Hard/Soft type
- **Notes:** `task.notes` (shows SKIPPED, RATE_LIMITED reasons)
- **Blocked by:** if blocked, which specific deps are unsatisfied

### 2.3 Real prompt preview in `details_panel.rs`

Replace static placeholder in `render_prompt_view()`:

```rust
// Instead of hardcoded text:
let prompt = PromptBuilder::new(50000).build_prompt(
    task, session, plan, &recent_iterations, leindex_context, memory_context
)?;
// Render the actual prompt text with syntax highlighting
```

This requires loading session and plan data for the selected task. Cache to avoid disk I/O in render loop.

### 2.4 Dashboard task breakdown

In `dashboard.rs`, compute and display:
```
Tasks: 5✓  3○  2⊘  (done / actionable / blocked)
```

---

## Phase 3: Memory System Integration (3-4 days)

**Goal:** Close E2-E6. Expose MemoryService through the Conductor.

### 3.1 Add MemoryService to ConductorPane

```rust
pub struct ConductorPane {
    // ... existing ...
    pub memory_service: Option<leindex_core::memory::MemoryService>,
    pub memory_modal: InputModal,         // For storing new memories
    pub memory_browser_visible: bool,
    pub memory_browser_items: Vec<leindex_core::memory::models::Memory>,
    pub memory_browser_selected: usize,
    pub memory_search_query: String,
    pub memory_category_filter: Option<MemoryCategory>,
}
```

Initialize in `ConductorPane::new()`:
```rust
memory_service: leindex_core::memory::MemoryService::new(None).ok(),
```

### 3.2 Memory browser overlay (`memory_browser.rs`)

**New file:** `crates/cockpit/src/conductor/memory_browser.rs`

Full-screen overlay (like dashboard) showing:
- Search bar at top
- Category filter tabs: All | Knowledge | Decision | Pattern | Context | ...
- Memory list with content preview, category icon, importance, timestamp
- Keybindings: `q` close, `/` search, `n` new memory, `d` delete, `Tab` cycle category

### 3.3 Store memory from TUI

Keybinding `Shift+M` opens memory store modal:
```
┌─ Store Memory ──────────────────────────┐
│ Category: [Decision ▼]                  │
│ Content:                                │
│ ┌──────────────────────────────────────┐│
│ │ Found that auth module uses JWT...   ││
│ └──────────────────────────────────────┘│
│ [Enter] Store  [Esc] Cancel             │
└─────────────────────────────────────────┘
```

On submit: `memory_service.store_memory(&content, category)`

### 3.4 Memory keybindings

```rust
(KeyModifiers::SHIFT, KeyCode::Char('M')) => {
    // Open memory browser
    pane.memory_browser_visible = true;
    pane.refresh_memory_list();
    ConductorAction::Handled
}
```

---

## Phase 4: Parallel Execution UI (4-5 days)

**Goal:** Close G7-G11. Build the Conductor-side UI components. These render from state populated by parallel engine events (Phase 6 wires events, but UI can be built and tested with mock state).

### 4.1 Add parallel state types to `model.rs`

```rust
pub enum ParallelStatus { Inactive, Running, Paused, MergeInProgress, Completed }
pub enum WorkerStatus { Idle, Running, Completed, Failed }
pub enum MergeStatus { Queued, InProgress, Completed, Conflicted, Failed, RolledBack }

pub struct ParallelWorkerState { worker_id, task_id, task_title, status, iteration, max_iterations, branch }
pub struct MergeQueueEntry { operation_id, task_id, status }
pub struct ConflictInfo { task_id, task_title, files: Vec<ConflictFile> }
pub struct ConflictFile { path, resolved, resolution_method }
```

Add to `ConductorState`:
```rust
pub parallel_status: ParallelStatus,
pub parallel_workers: Vec<ParallelWorkerState>,
pub merge_queue: Vec<MergeQueueEntry>,
pub active_conflict: Option<ConflictInfo>,
pub current_parallel_group: usize,
pub total_parallel_groups: usize,
pub selected_conflict_index: usize,
```

### 4.2 Create `parallel_view.rs`

Worker list with merge queue. Replaces logs pane when parallel is active.

### 4.3 Create `conflict_panel.rs`

Modal overlay for merge conflict resolution. Per-file navigation with R/S/O/T keybindings.

### 4.4 Add parallel ConductorEvent variants + state_machine transitions

12 new event variants for parallel lifecycle. Each gets a transition handler.

### 4.5 Extend keybindings for parallel control

Conflict resolution keys gated on `active_conflict.is_some()`. Parallel pause/resume on `Ctrl+P`.

### 4.6 Update header, footer, dashboard for parallel mode

Header: `∥ W:3 G:2/5` indicator. Footer: context-sensitive keys. Dashboard: worker/merge metrics.

---

## Phase 5: Parallel Event Polling (2-3 days)

**Goal:** Close B6. Wire engine parallel events through the existing polling infrastructure.

### 5.1 Add parallel EngineEvent variants (`orchestrate/control.rs`)

```rust
// Add to EngineEvent enum:
ParallelStarted { session_id: String, worker_count: usize, group_count: usize, timestamp: String },
WorkerStarted { worker_id: String, task_id: String, worktree_path: String, timestamp: String },
WorkerProgress { worker_id: String, iteration: u64, timestamp: String },
WorkerCompleted { worker_id: String, success: bool, timestamp: String },
MergeQueued { operation_id: String, task_id: String, timestamp: String },
MergeStarted { operation_id: String, timestamp: String },
MergeConflicted { operation_id: String, files: Vec<String>, timestamp: String },
MergeCompleted { operation_id: String, timestamp: String },
ConflictResolved { file: String, method: String, timestamp: String },
ParallelCompleted { total_tasks: usize, total_merges: usize, timestamp: String },
```

### 5.2 Add parallel ControlCommand variants (`orchestrate/control.rs`)

```rust
PauseParallel,
ResumeParallel,
ResolveConflict { file: String, method: String },
```

### 5.3 Extend `polling.rs::process_engine_event()`

Map each new `EngineEvent` variant → `ConductorEvent` → `state.transition()` → `BUS.broadcast()`. Follow the exact pattern of existing event processing.

---

## Phase 6: Pi-Mono Deep Integration + Polish (3-4 days)

**Goal:** Close M3-M9, I2-I3, J3, H5.

### 6.1 Pi-Mono start options

Extend `pane.rs::get_start_command()` to optionally include `--pi-agent`, `--pi-chain`, `--pi-parallel` flags.

Add a "Start with Pi-Mono" selector modal:
```
┌─ Start Orchestration ──────────────┐
│ Mode:     ○ Standard  ● Pi-Mono    │
│ Pi Mode:  ○ Single  ○ Chain  ○ Par │
│ Agent:    [scout ▼]                │
│ [Enter] Start  [Esc] Cancel        │
└────────────────────────────────────┘
```

### 6.2 Pi-Mono status in dashboard

Poll `maestro_pi_mono::PiDetection::detect()` and show:
```
Pi-Mono:       Available (4 agents configured)
```

### 6.3 Memory dashboard stats

Poll `memory_service.stats()` and show:
```
Memories:      42 total (12 knowledge, 8 decisions, 22 context)
```

### 6.4 Iteration history enrichment

- Show timing when available (parse `started_at`/`completed_at` from IterationLog)
- Clickable rows: pressing Enter on a history item loads that iteration's output into the Output view

### 6.5 Subagent tree wiring

Option A (OMP-based): In `polling.rs`, when OMP is available, poll `omp_manager.get_agent_status()` and populate `state.subagents` from active OMP workers.

Option B (Engine events): Add `EngineEvent::SubagentStarted/Completed` to engine, emit from `runner.rs` when subagent invocations are detected.

### 6.6 `maestro implement` from TUI

Keybinding `n` (new track) → opens implement modal:
```
┌─ New Implementation ───────────────┐
│ Command: maestro implement         │
│ Description: [________________]    │
│ Tool: [claude ▼]                   │
│ Session: ○ Ask  ● New  ○ Current   │
│ [Enter] Launch  [Esc] Cancel       │
└────────────────────────────────────┘
```

Spawns `maestro implement` as detached process.

---

## Phase 7: Theme System + Remaining Polish (2-3 days)

**Goal:** Close K2-K3, J4, remaining low-priority items.

### 7.1 Additional themes

Add to `theme.rs`:
```rust
pub fn from_name(name: &str) -> Self {
    match name {
        "tokyo-night" => Self::tokyo_night(),   // existing default
        "catppuccin" => Self::catppuccin(),
        "dracula" => Self::dracula(),
        "high-contrast" => Self::high_contrast(),
        "solarized-light" => Self::solarized_light(),
        _ => Self::default(),
    }
}
```

### 7.2 Theme selector

Keybinding `T` → `SelectorModal` with theme names. Selection updates `ConductorTheme` and persists to `~/.maestro/conductor.toml`.

### 7.3 Toast notification system

Lightweight toast queue rendered in bottom-right corner:
```rust
pub struct ToastQueue {
    toasts: VecDeque<Toast>,
    max_visible: usize,
}
pub struct Toast {
    message: String,
    level: ToastLevel,    // Info/Warning/Error
    created_at: Instant,
    ttl: Duration,
}
```

Replace `ConductorAction::StatusMessage` emissions with `toast_queue.push()`. Render as small overlay.

---

## Files Summary

### New Files (8)

```
crates/cockpit/src/conductor/
├── input_modal.rs         # Reusable text input overlay
├── selector_modal.rs      # Reusable list selector overlay
├── memory_browser.rs      # Memory list/search/store overlay
├── parallel_view.rs       # Worker list + merge queue display
├── conflict_panel.rs      # Merge conflict resolution overlay
├── toast.rs               # Toast notification queue
```

### Modified Files — Conductor (12)

```
crates/cockpit/src/conductor/
├── mod.rs                 # Register new modules
├── model.rs               # Parallel state types, new ConductorEvent variants
├── pane.rs                # Add modal fields, wire new views into render
├── state_machine.rs       # Parallel + steering event transitions
├── keybindings.rs         # Steering, config, parallel, memory keybindings
├── polling.rs             # Process parallel engine events
├── track_tree.rs          # Blocked/actionable indicators
├── details_panel.rs       # Task deps, description, notes, real prompt preview
├── dashboard.rs           # Task breakdown, parallel metrics, memory stats
├── header.rs              # Parallel mode indicator
├── footer.rs              # Context-sensitive keys per mode
├── iteration_history.rs   # Timing, worker attribution, drill-down
└── theme.rs               # Additional palettes + from_name()
```

### Modified Files — Engine (2)

```
maestro/leindex/rust/src/orchestrate/
├── control.rs             # New ControlCommand + EngineEvent variants
└── engine.rs              # Handle new commands (steering, config, parallel)
```

---

## Dependency Graph

```
Phase 1 (UI primitives + steering)  ←── No dependencies, START HERE
    ↓
Phase 2 (task display)              ←── Needs Phase 1 (for details enrichment patterns)
    ↓
Phase 3 (memory integration)        ←── Needs Phase 1 (input_modal)
    ↓
Phase 4 (parallel UI)               ←── Needs Phase 1 (selector_modal, input_modal)
    ↓
Phase 5 (parallel event polling)     ←── Needs Phase 4 (state types) + engine parallel executor
    ↓
Phase 6 (pi-mono + polish)          ←── Needs Phases 1-3
    ↓
Phase 7 (themes + toast)            ←── Needs Phase 1 (selector_modal)
```

**Phases 1-4 are parallelizable across developers.** Phase 5 is blocked on the engine-side parallel executor. Phases 2, 3, 6, 7 have no engine dependencies.

---

## Timeline

| Phase | Duration | Engine Work? | Blocks on Engine Parallel? |
|-------|----------|-------------|---------------------------|
| 1. UI Primitives + Steering | 4-5 days | Yes (3 control commands) | No |
| 2. Task Display & Details | 2-3 days | No | No |
| 3. Memory Integration | 3-4 days | No | No |
| 4. Parallel UI | 4-5 days | No (mock state) | No |
| 5. Parallel Polling | 2-3 days | Yes (event variants) | **Yes** |
| 6. Pi-Mono + Polish | 3-4 days | No | No |
| 7. Themes + Toast | 2-3 days | No | No |
| **Total** | **20-27 days** | | |

Phases 1-4 can reach **~80% parity** without any engine-side parallel executor. Phase 5 is the only phase blocked on the parallel engine work.
