# Conductor Tab Gap Analysis: Maestro vs Ralph-TUI — Target 100%

**Analysis Date:** 2026-02-13 (Revised v2)  
**Maestro Conductor Location:** `crates/cockpit/src/conductor/` (20 files)  
**Orchestrate Engine:** `maestro/leindex/rust/src/orchestrate/` (14 files)  
**CLI Entry Point:** `crates/cli/src/main.rs` — `maestro orchestrate {start|pause|resume|abort|status|list}`  
**Memory System:** `maestro/leindex/rust/src/memory/` (MemoryService, Turso backend)

---

## Target: 100% Ralph-TUI Feature Parity + Maestro-Specific Deep Integration

This analysis covers every feature category in Ralph-TUI and maps each to the Conductor pane's current state, identifying the exact delta to reach 100%. It also covers deep CLI integration (orchestrate commands, steering, memory) that Ralph-TUI handles but the Conductor does not yet expose.

---

## Feature Matrix (Comprehensive)

### A. Core Orchestration Control

| # | Feature | Ralph-TUI | Conductor | Status | Gap |
|---|---------|-----------|-----------|--------|-----|
| A1 | Start orchestrate loop | ✅ | ✅ keybinding `s` → spawns `maestro orchestrate start` | **Done** | — |
| A2 | Pause loop | ✅ | ✅ keybinding `p` → spawns `maestro orchestrate pause` | **Done** | — |
| A3 | Resume loop | ✅ | ✅ keybinding `r` → spawns `maestro orchestrate resume` | **Done** | — |
| A4 | Abort loop | ✅ | ✅ keybinding `Ctrl+A` → `ControlCommand::Abort` via control.json | **Done** | — |
| A5 | Status check | ✅ | ✅ keybinding `?` → spawns `maestro orchestrate status` | **Done** | — |
| A6 | Retry current task | ✅ | ✅ `Ctrl+R` → `ControlCommand::Retry` | **Done** | — |
| A7 | Skip current task | ✅ | ✅ `Ctrl+S` → `ControlCommand::Skip` | **Done** | — |
| A8 | Set error strategy | ✅ | ⚠️ `ControlCommand::SetErrorStrategy` exists but no keybinding | **Gap** | Add keybinding |
| A9 | Set max iterations | ✅ | ❌ No control command, no keybinding | **Gap** | Add `ControlCommand::SetMaxIterations` + keybinding |
| A10 | Switch loop mode (plan/build) | ✅ | ❌ Mode shown in header but not switchable at runtime | **Gap** | Add `ControlCommand::SetMode` + keybinding |
| A11 | Steering message injection | ✅ | ❌ No ability to inject a user message into the next iteration prompt | **Gap** | Add `ControlCommand::Steer` + input modal |
| A12 | Tool/agent selection at runtime | ✅ | ❌ Agent shown in details but not switchable mid-session | **Gap** | Add `ControlCommand::SwitchAgent` + selector |

### B. State Machine & Event Architecture

| # | Feature | Ralph-TUI | Conductor | Status | Gap |
|---|---------|-----------|-----------|--------|-----|
| B1 | FSM with valid transitions | ✅ | ✅ 10 states in `state_machine.rs` | **Done** | — |
| B2 | Event bus (decouple poll/render) | ✅ | ✅ `TelemetryBus` via tokio broadcast | **Done** | — |
| B3 | Bidirectional control channel | ✅ | ✅ `control.json` (commands) + `events.jsonl` (events) | **Done** | — |
| B4 | Typed engine events | ✅ | ✅ 6 `EngineEvent` variants consumed in `polling.rs` | **Done** | — |
| B5 | Typed conductor events | ✅ | ✅ 18 `ConductorEvent` variants | **Done** | — |
| B6 | Parallel execution events | ✅ | ❌ No parallel event types | **Gap** | Add ~12 new event variants |

### C. Task & Track Display

| # | Feature | Ralph-TUI | Conductor | Status | Gap |
|---|---------|-----------|-----------|--------|-----|
| C1 | Track tree with expand/collapse | ✅ | ✅ `track_tree.rs` with ▶/▼ | **Done** | — |
| C2 | Task hierarchy rendering | ✅ | ✅ Nested tasks with `[+]/[-]` | **Done** | — |
| C3 | Status badges (✓/▶/○/✗) | ✅ | ✅ Full set in `theme.rs` | **Done** | — |
| C4 | Master track indicator | ✅ | ✅ 👑 icon, yellow highlight | **Done** | — |
| C5 | External session discovery | ✅ | ✅ Scans `~/.maestro/orchestrate/`, labels `(ext)` | **Done** | — |
| C6 | Runtime status per track | ✅ | ✅ `track_runtime_statuses` HashMap, polled periodically | **Done** | — |
| C7 | Iteration count on track | ✅ | ✅ Shows `(N)` next to active track | **Done** | — |
| C8 | Blocked task indicator (⊘) | ✅ | ❌ `STATUS_BLOCKED` defined in theme but **never used** in tree | **Gap** | Wire in `track_tree.rs` |
| C9 | Actionable task indicator | ✅ | ❌ `STATUS_ACTIONABLE` defined but **never used** | **Gap** | Wire in `track_tree.rs` |
| C10 | Task dependency display in details | ✅ | ❌ Details shows title/ID/status only, not deps | **Gap** | Add dep list to `details_panel.rs` |
| C11 | Task description in details | ✅ | ❌ `task.description` not shown in Task view | **Gap** | Add to `details_panel.rs` |
| C12 | Task notes display | ✅ | ❌ `task.notes` (SKIPPED, RATE_LIMITED) not shown | **Gap** | Add to `details_panel.rs` |
| C13 | Parallel group indicators | ✅ | ❌ Not applicable yet | **Gap** | Add when parallel lands |

### D. Details Panel & Views

| # | Feature | Ralph-TUI | Conductor | Status | Gap |
|---|---------|-----------|-----------|--------|-----|
| D1 | Details view (task metadata) | ✅ | ✅ Alt+1 → track info, agent, commands | **Done** | — |
| D2 | Output view (live stdout/stderr) | ✅ | ✅ Alt+2 → scrollable iteration output | **Done** | — |
| D3 | Prompt preview | ✅ | ⚠️ Alt+3 → static placeholder, doesn't render actual prompt | **Gap** | Call `PromptBuilder` to render real prompt |
| D4 | Worker detail view | ✅ | ❌ No Alt+4 worker view | **Gap** | Add `DetailsViewMode::WorkerDetail` |
| D5 | Agent activity display | ✅ | ✅ Tool, model, since time | **Done** | — |
| D6 | Rate limit display | ✅ | ✅ Backoff countdown, retry count | **Done** | — |
| D7 | Last iteration summary | ✅ | ✅ Shows status + output preview | **Done** | — |
| D8 | Memories display | ✅ | ✅ `track_memories` rendered with category icons | **Done** | — |
| D9 | LSP diagnostics display | ✅ | ✅ Errors/warnings in state, stderr annotation | **Done** | — |

### E. Memory System Integration

| # | Feature | Ralph-TUI | Conductor | Status | Gap |
|---|---------|-----------|-----------|--------|-----|
| E1 | Display track memories | N/A (Maestro-specific) | ✅ Polled and shown in details panel | **Done** | — |
| E2 | Store memory from conductor | ❌ | ❌ No ability to create a memory from TUI | **Gap** | Add input modal + `MemoryService::store_memory` |
| E3 | Search memories | ❌ | ❌ No memory search in TUI | **Gap** | Add search input + `MemoryService::search_memories` |
| E4 | Memory browser/list view | ❌ | ❌ Only last few shown in details | **Gap** | Add memory list overlay (like dashboard) |
| E5 | Memory category filter | ❌ | ❌ No filtering | **Gap** | Filter by category in list view |
| E6 | Delete/expire memory | ❌ | ❌ No delete capability | **Gap** | Add keybinding in memory view |
| E7 | Memory injection into prompt | ✅ (via engine) | ✅ Engine injects via `PromptBuilder` | **Done** (engine-side) | — |

### F. Steering & Runtime Configuration

| # | Feature | Ralph-TUI | Conductor | Status | Gap |
|---|---------|-----------|-----------|--------|-----|
| F1 | Steering message (inject user text into next iteration) | ✅ | ❌ No control command or UI | **Gap (Critical)** | Add `ControlCommand::Steer` + text input modal |
| F2 | Change max iterations at runtime | ✅ | ❌ | **Gap** | Add `ControlCommand::SetMaxIterations` |
| F3 | Change error strategy at runtime | ✅ | ⚠️ `ControlCommand::SetErrorStrategy` exists, no keybinding | **Gap** | Add keybinding/selector |
| F4 | Change agent/model at runtime | ✅ | ❌ | **Gap** | Add `ControlCommand::SwitchAgent` |
| F5 | Toggle sandbox mode | ✅ | ❌ Displayed but not toggleable | **Gap** | Add `ControlCommand::ToggleSandbox` |
| F6 | Toggle dangerous mode | ✅ | ❌ Displayed but not toggleable | **Gap** | Add `ControlCommand::ToggleDangerous` |
| F7 | Config push/pull (remote) | ✅ | ❌ | **Gap** | Phase 5 (remote) |

### G. Parallel Execution System

| # | Feature | Ralph-TUI | Conductor | Status | Gap |
|---|---------|-----------|-----------|--------|-----|
| G1 | ParallelExecutor coordination | ✅ | ❌ | **Gap (Critical)** | Engine + Conductor |
| G2 | TaskGraphAnalysis (Kahn's) | ✅ | ❌ (deps parsed, no graph analysis) | **Gap** | Engine-side + viz |
| G3 | Worker abstraction | ✅ | ❌ | **Gap** | Engine-side |
| G4 | WorktreeManager (git worktrees) | ✅ | ❌ | **Gap** | Engine-side |
| G5 | MergeEngine (sequential merge queue) | ✅ | ❌ | **Gap** | Engine-side |
| G6 | ConflictResolver (AI-assisted) | ✅ | ❌ | **Gap** | Engine-side |
| G7 | ParallelProgressView (workers + queue) | ✅ | ❌ | **Gap** | Conductor UI |
| G8 | WorkerDetailView | ✅ | ❌ | **Gap** | Conductor UI |
| G9 | MergeProgressView | ✅ | ❌ | **Gap** | Conductor UI |
| G10 | ConflictResolutionPanel | ✅ | ❌ | **Gap** | Conductor UI |
| G11 | Parallel control keybindings | ✅ | ❌ | **Gap** | Conductor keybindings |

### H. Header, Footer & Dashboard

| # | Feature | Ralph-TUI | Conductor | Status | Gap |
|---|---------|-----------|-----------|--------|-----|
| H1 | Status bar (header) | ✅ | ✅ Status/track/task/agent/iteration/progress/OMP | **Done** | — |
| H2 | Keybinding footer | ✅ | ✅ Context-aware key hints | **Done** | — |
| H3 | Dashboard overlay | ✅ | ✅ Session ID, git, agents, rate limit, model, uptime | **Done** | — |
| H4 | Parallel metrics in dashboard | ✅ | ❌ No worker/merge queue metrics | **Gap** | Add when parallel lands |
| H5 | Task breakdown in dashboard | ✅ | ❌ Only completed count, no actionable/blocked breakdown | **Gap** | Add |

### I. Iteration History

| # | Feature | Ralph-TUI | Conductor | Status | Gap |
|---|---------|-----------|-----------|--------|-----|
| I1 | Iteration history list | ✅ | ✅ Status icon + iter number + task ID | **Done** | — |
| I2 | Iteration timing info | ✅ | ❌ Duration not shown (not in IterationLog) | **Gap** | Add timing to log display |
| I3 | Iteration output drill-down | ✅ | ⚠️ Output is flat; no per-iteration navigation | **Gap** | Add click-to-view per iteration |
| I4 | Parallel iteration attribution | ✅ | ❌ No worker ID per iteration | **Gap** | Add when parallel lands |

### J. UI Components & Chrome

| # | Feature | Ralph-TUI | Conductor | Status | Gap |
|---|---------|-----------|-----------|--------|-----|
| J1 | Modal overlay pattern | ✅ | ✅ Dashboard + project_selector use `centered_rect` + `Clear` | **Done** | — |
| J2 | Project selector | ✅ | ✅ Full implementation with auto-discovery | **Done** | — |
| J3 | Subagent tree | ✅ | ⚠️ UI exists, data never populated | **Gap** | Wire engine events or OMP status |
| J4 | Toast notification system | ✅ | ❌ `StatusMessage` only | **Gap** | Add toast queue + overlay |
| J5 | Text input modal (for steering) | ✅ | ❌ No text input capability in TUI | **Gap (Critical for steering)** | Add |
| J6 | Selector modal (for agent/strategy) | ✅ | ❌ No list-selector modal | **Gap** | Add |
| J7 | File browser component | ✅ | ❌ | **Gap** | Low priority |
| J8 | Image attachments | ✅ | ❌ | **Gap** | Low priority |

### K. Theme System

| # | Feature | Ralph-TUI | Conductor | Status | Gap |
|---|---------|-----------|-----------|--------|-----|
| K1 | Theme structure | ✅ | ✅ `ConductorTheme` with bg/fg/status/task/accent groups | **Done** | — |
| K2 | Multiple themes | ✅ 5 themes | ❌ 1 theme only (Tokyo Night) | **Gap** | Add 4 more palettes |
| K3 | Theme selector | ✅ | ❌ | **Gap** | Add keybinding + selector |

### L. Remote Orchestration

| # | Feature | Ralph-TUI | Conductor | Status | Gap |
|---|---------|-----------|-----------|--------|-----|
| L1 | Local control plane | ✅ | ✅ File-based IPC (control.json/events.jsonl) | **Done** | — |
| L2 | WebSocket server | ✅ | ❌ | **Gap** | Network remote control |
| L3 | Remote client with auto-reconnect | ✅ | ❌ | **Gap** | Network remote control |
| L4 | Token-based auth | ✅ | ❌ | **Gap** | Network remote control |

### M. Pi-Mono / CLI Deep Integration

| # | Feature | Description | Conductor | Status | Gap |
|---|---------|-------------|-----------|--------|-----|
| M1 | `maestro orchestrate start` via TUI | Start with tool/model/mode/retries/sandbox/dangerous | ✅ Spawns process | **Done** | — |
| M2 | `maestro orchestrate pause/resume/abort` | Control running session | ✅ Via spawned process + control.json | **Done** | — |
| M3 | Pi-Mono single agent from TUI | `--pi-agent scout` | ❌ | **Gap** | Add pi-mono options to start command builder |
| M4 | Pi-Mono chain from TUI | `--pi-chain scout,planner,worker` | ❌ | **Gap** | Add chain mode selector |
| M5 | Pi-Mono parallel from TUI | `--pi-parallel worker,worker` | ❌ | **Gap** | Add parallel mode selector |
| M6 | Pi-Mono agent status in dashboard | Show active pi-mono agents | ❌ | **Gap** | Poll pi-mono status |
| M7 | `maestro memory store` from TUI | Store observation/decision memory | ❌ | **Gap** | Text input → memory store |
| M8 | `maestro memory status` in dashboard | Show memory DB stats | ❌ | **Gap** | Add to dashboard overlay |
| M9 | `maestro implement` from TUI | Initiate track implementation | ❌ | **Gap** | Add keybinding to spawn implement |

---

## Score Summary

| Category | Items | Done | Gap | % |
|----------|-------|------|-----|---|
| A. Core Orchestration Control | 12 | 7 | 5 | 58% |
| B. State Machine & Events | 6 | 5 | 1 | 83% |
| C. Task & Track Display | 13 | 7 | 6 | 54% |
| D. Details Panel & Views | 9 | 8 | 1 | 89% |
| E. Memory System | 7 | 3 | 4 | 43% |
| F. Steering & Config | 7 | 0 | 7 | 0% |
| G. Parallel Execution | 11 | 0 | 11 | 0% |
| H. Header/Footer/Dashboard | 5 | 3 | 2 | 60% |
| I. Iteration History | 4 | 1 | 3 | 25% |
| J. UI Components | 8 | 3 | 5 | 38% |
| K. Theme System | 3 | 1 | 2 | 33% |
| L. Remote Orchestration | 4 | 1 | 3 | 25% |
| M. Pi-Mono / CLI Integration | 9 | 2 | 7 | 22% |
| **TOTAL** | **98** | **41** | **57** | **42%** |

---

## Priority-Ordered Gap Closure Roadmap

### Tier 1 — Critical (Enables core Ralph-TUI parity) (Target: +25%)

1. **F1: Steering message injection** — This is the #1 missing feature. Users need to inject guidance ("focus on error handling", "skip tests for now") into the next iteration. Requires:
   - `ControlCommand::Steer { message: String }` in engine control.rs
   - Engine reads steering and prepends to next prompt
   - Conductor: text input modal (J5) + keybinding (`Ctrl+M` for message)

2. **C8-C12: Task display improvements** — Wire existing data into existing views:
   - `STATUS_BLOCKED`/`STATUS_ACTIONABLE` in track_tree.rs
   - Dependencies, description, notes in details_panel.rs
   - Zero engine changes, pure Conductor-side

3. **E2-E4: Memory CRUD from TUI** — Store, search, browse memories:
   - `MemoryService` already has `store_memory()`, `search_memories()`, `list_memories()`
   - Need text input modal (J5) + memory list overlay
   - Keybinding: `m` for memory browser, `Ctrl+M` conflicts with steering → use `M` (shift)

4. **A9-A11: Runtime configuration** — Change max iterations, mode, inject steering:
   - New `ControlCommand` variants
   - Engine reads them in control loop
   - Conductor: selector modals (J6)

5. **D3: Real prompt preview** — Details panel Alt+3 shows placeholder. Should call `PromptBuilder::build_prompt()` with current task/session/plan to show actual prompt.

### Tier 2 — High (Completes interaction model) (Target: +20%)

6. **A12, F3-F6: Runtime agent/config switching** — Switch agent, toggle sandbox/dangerous, change error strategy mid-session.

7. **J5-J6: Input modals** — Text input and list selector are prerequisites for steering, memory, and config changes. These are reusable primitives.

8. **G7-G11: Parallel UI components** — ParallelProgressView, ConflictPanel, parallel keybindings. Blocked on engine parallel executor but UI can be built against mock events.

9. **J3: Subagent tree wiring** — Either:
   - Add `EngineEvent::SubagentStarted/Completed` to engine
   - Or populate from OMP agent status (already partially wired)

10. **I2-I3: Iteration history enrichment** — Timing, per-iteration drill-down.

### Tier 3 — Medium (Polish & Power Features) (Target: +10%)

11. **G1-G6: Engine-side parallel execution** — ParallelExecutor, WorktreeManager, MergeEngine, ConflictResolver. Engine work, not Conductor-side.

12. **M3-M6: Pi-Mono deep integration** — Start with pi-agent/pi-chain flags, show pi-mono status.

13. **H4-H5: Dashboard enrichment** — Parallel metrics, task breakdown (actionable/blocked/done).

14. **K2-K3: Theme system** — Add catppuccin, dracula, high-contrast, solarized-light palettes + selector.

### Tier 4 — Low (Nice-to-have) (Target: +5%)

15. **L2-L4: Remote orchestration** — WebSocket server for network control.
16. **J4: Toast notifications** — Auto-dismiss notification queue.
17. **J7: File browser** — Directory navigation + file preview.
18. **J8: Image attachments** — Clipboard paste + image storage.
19. **M7-M9: Remaining CLI integration** — Memory store CLI, implement from TUI.

---

## Architecture Notes for Gap Closure

### Control Command Extensions (Engine-side, `orchestrate/control.rs`)

```rust
// New ControlCommand variants needed:
Steer { message: String },                    // F1
SetMaxIterations { max: u64 },                // A9/F2
SetMode { mode: String },                     // A10
SwitchAgent { tool: String, model: Option<String> },  // A12/F4
ToggleSandbox,                                // F5
ToggleDangerous,                              // F6
// Parallel (Phase 3+):
PauseParallel,
ResumeParallel,
ResolveConflict { file: String, method: String },
```

### Engine Prompt Injection Point (`orchestrate/engine.rs`)

The steering message should be injected in `build_iteration_prompt()` between the task context and instructions:

```rust
// In build_iteration_prompt(), after memory_context:
if let Some(steering) = self.check_steering_message(track_id)? {
    prompt.push_str("\n## User Steering Message\n\n");
    prompt.push_str(&steering);
    prompt.push('\n');
}
```

### Conductor Input Modal Pattern (New `conductor/input_modal.rs`)

```rust
pub struct InputModal {
    pub title: String,
    pub prompt: String,
    pub input: String,
    pub cursor_pos: usize,
    pub visible: bool,
    pub on_submit: Box<dyn FnOnce(String)>,
}
```

This is the key missing UI primitive. Once built, it unlocks:
- Steering message input (F1)
- Memory store input (E2)
- Max iterations input (A9)
- Any freeform text input

### Conductor Selector Modal Pattern (New `conductor/selector_modal.rs`)

```rust
pub struct SelectorModal<T> {
    pub title: String,
    pub items: Vec<(String, T)>,
    pub selected: usize,
    pub visible: bool,
}
```

Unlocks:
- Agent/model selection (A12)
- Error strategy selection (F3)
- Theme selection (K3)
- Pi-Mono agent selection (M3)

---

## What's Already Excellent

- **State machine** — 10-state FSM with proper transition guards
- **Event architecture** — Typed events, broadcast bus, structured JSONL
- **Polling infrastructure** — Throttled multi-track polling with catch-up suppression
- **Command generation** — `get_start_command()`, `get_pause_command()` build proper CLI invocations
- **Control channel** — Atomic file I/O with temp+rename pattern
- **Theme semantics** — 25+ semantic color slots covering all UI states
- **OMP integration** — Agent manager with per-track workers (Maestro-specific advantage)
- **Project discovery** — Auto-discovers from CWD, tmux, and `~/.maestro/orchestrate/`
- **Test coverage** — Unit tests for track detection, external discovery, output suppression, command generation
