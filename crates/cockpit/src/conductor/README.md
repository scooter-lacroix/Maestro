# Maestro Cockpit Conductor

The Conductor module provides autonomous loop execution and telemetry visualization for Maestro tracks, inspired by Ralph TUI.

## Features

### 1. Unified Session Discovery
- **Project Tracks**: Automatically loads tracks defined in the current project's `tracks.md`.
- **External Sessions**: Scans `~/.maestro/orchestrate/` for active sessions initiated via CLI or agents (`/maestro:implement`). External tracks are labeled with `(ext)`.

### 2. Telemetry Tree
- **Master Tracks**: Identified by `master` in ID or metadata, rendered with a 👑 icon.
- **Track Hierarchy**: Supports Master Track → Track → Task hierarchy.
- **Status Badges**:
  - `[ ]` Pending
  - `↻` Running
  - `[P]` Paused
  - `[F]` Failed
  - `[x]` Completed
- **Iteration Count**: Shows the current iteration number next to active tracks.

### 3. Loop Control
- **Start (s)**: Launches the orchestrate loop for the selected track.
- **Pause (p)**: Requests a pause after the current iteration.
- **Resume (r)**: Resumes a paused session.
- **Status (?)**: Checks the current status of the orchestrate engine.

### 4. Live Telemetry & Logs
- **Details Panel**: Shows active agent activity (tool, model, uptime), current loop iteration, and rate limit status (with backoff countdown).
- **Iteration History**: Surfaces results of the last 50 iterations for the selected track.
- **Runtime Logs**: Streams live stdout/stderr from the agent loop in the bottom-left pane.

## Known Limitations
- **Task Hierarchy for External Tracks**: External tracks (not in current `tracks.md`) lack detailed task hierarchy as the project path is not yet stored in `session.json`. They appear as a single top-level node.
- **Process Spawning**: Commands are spawned as detached processes using the `maestro` binary. Ensure `maestro` is in your `$PATH`.

## Developer Notes
- **Telemetry BUS**: Internal events are broadcast via `tokio::sync::broadcast` to decouple polling from rendering.
- **Polling Throttling**: Global track status is polled every 2 seconds (4th frame) to minimize disk I/O, while the selected track is polled every 500ms for live updates.
- **Catch-up Suppression**: When switching tracks, historical output from `iterations.jsonl` is suppressed to prevent UI flooding, while populating the history list.
