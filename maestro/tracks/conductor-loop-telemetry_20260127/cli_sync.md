# CLI Sync & Discovery Documentation

## Overview
The Cockpit Conductor is designed to be session-aware across different invocation methods (TUI, CLI, and Agent commands). This is achieved through a common telemetry store in `~/.maestro/orchestrate/`.

## Discovery Mechanism
The Conductor pane performs a dual-source discovery:
1. **Project Tracks**: Loads all tracks defined in the current project's `tracks.md`.
2. **External Sessions**: Scans `~/.maestro/orchestrate/` for any directory containing a `session.json`.

If an external session is found that is NOT in the current `tracks.md`, it is added to the UI tree with an `(ext)` label.

## Parity Status
| Feature | TUI-Started | CLI-Started (`maestro orchestrate`) | Agent-Started (`/maestro:implement`) |
| :--- | :--- | :--- | :--- |
| **Visibility** | ✅ Yes | ✅ Yes | ✅ Yes (via telemetry store) |
| **Status (Running/Paused)** | ✅ Live | ✅ Live | ✅ Live |
| **Iteration Count** | ✅ Live | ✅ Live | ✅ Live |
| **Log Streaming** | ✅ Live | ✅ Live | ✅ Live |
| **Task Tree (Sub-tasks)** | ✅ Detailed | ⚠️ Limited* | ⚠️ Limited* |

### *Limitations & Rationale
1. **Missing Task Hierarchy for External Tracks**: 
   - **Limitation**: CLI or Agent-started sessions often target tracks that are not part of the *currently opened* project in Cockpit. 
   - **Rationale**: Since the Conductor only knows how to parse the `plan.md` of tracks it has a path to, and external sessions only store their `track_id` in `session.json`, the Cockpit cannot always locate the corresponding `plan.md` to render the full task tree for external tracks.
   - **Result**: External tracks show as a single top-level node with global status/logs but no expandable task list.

2. **Command Re-attachment**:
   - **Limitation**: Detaching/Re-attaching to a CLI session's tmux pane is not directly handled by the Conductor tab (use the `Sessions` tab for that).
   - **Rationale**: The Conductor focuses on autonomous loop telemetry, not interactive terminal multiplexing.

3. **Log History Cap**:
   - **Limitation**: History is capped at the last 50 iterations for external sessions to preserve TUI performance.
