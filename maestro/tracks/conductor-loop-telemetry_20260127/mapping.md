# Ralph to Maestro Mapping - Conductor Loop & Telemetry

## Core Engine & State

| Component | Ralph (TypeScript) | Maestro (Rust) | Notes |
| :--- | :--- | :--- | :--- |
| Execution Engine | `src/engine/index.ts` | `leindex-core/src/orchestrate/engine.rs` | Maestro handles the loop lifecycle: select -> prompt -> run -> detect. |
| Session Persistence | `src/session/persistence.ts` | `leindex-core/src/orchestrate/state.rs` | Maestro uses `session.json` in `~/.maestro/orchestrate/<track_id>/`. |
| Iteration Logs | `src/logs/persistence.ts` | `leindex-core/src/orchestrate/polling.rs` | Maestro uses `iterations.jsonl`. |
| Rate Limit Detection | `src/engine/rate-limit-detector.ts` | `leindex-core/src/orchestrate/model.rs` | `RateLimitState` is part of `SessionState`. |
| Agent Config | `src/config/types.ts` | `leindex-core/src/orchestrate/model.rs` | `AgentConfig` (tool, model, dangerous, sandbox). |

## TUI Components (Cockpit Conductor)

| Component | Ralph (Inkt) | Cockpit (Ratatui) | Status in Cockpit |
| :--- | :--- | :--- | :--- |
| Track/Task Tree | `src/tui/components/LeftPanel.tsx` | `conductor/track_tree.rs` | Missing Master Track and loop state badges. |
| Details View | `src/tui/components/TaskDetailView.tsx` | `conductor/details_panel.rs` | Basic. Needs agent activity and loop status. |
| Iteration History | `src/tui/components/IterationHistoryView.tsx` | `conductor/iteration_history.rs` | Needs wiring to `iterations.jsonl`. |
| Subagent Tree | `src/tui/components/SubagentTreePanel.tsx` | `conductor/subagent_tree.rs` | Exists but occupies bottom-left where logs should be. |
| Live Logs | `src/tui/components/ProgressDashboard.tsx` | `conductor/details_panel.rs` (Output mode) | Needs to be moved to bottom-left pane. |
| Header/Status | `src/tui/components/Header.tsx` | `conductor/header.rs` | Mostly feature-complete. |
| Footer/Keybinds | `src/tui/components/Footer.tsx` | `conductor/footer.rs` | Needs implementation of active commands. |

## Event Sources

| Event | Ralph Mechanism | Maestro Mechanism |
| :--- | :--- | :--- |
| Iteration Started | Engine hook | `iterations.jsonl` append |
| Agent Output | Stream buffer | `iterations.jsonl` `output` field |
| Task Completion | Tracker detect | `session.json` `current_task_id` change |
| Loop Pause/Stop | Signal/Lock file | `session.json` `status` change |
