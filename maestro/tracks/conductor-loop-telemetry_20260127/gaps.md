# Audit Gaps - Cockpit Conductor Data Flow

## 1. Session Discovery
- **Current Behavior**: `ConductorPane` only polls tracks that are explicitly listed in the current project's `tracks.md` file.
- **Gap**: Sessions started via CLI (`maestro orchestrate start ...`) for tracks not in the current `tracks.md` are invisible. 
- **Required**: Auto-discovery of active orchestrate sessions by scanning `~/.maestro/orchestrate/` and adding them to the Conductor view, regardless of their presence in `tracks.md`.

## 2. Telemetry Tree & Nodes
- **Current Behavior**: Tree only shows `Track` and `Task` nodes. Status icons are simple badges.
- **Gap**: Missing `Master Track` hierarchy. No visibility into orchestrate loop state (iteration count, iteration status, loop enablement) at the node level.
- **Required**: Support for Master Track → Track → Task. Integrate loop telemetry (iteration count, active status) into tree rendering.

## 3. Telemetry Details
- **Current Behavior**: Basic track/task details. Commands are just strings.
- **Gap**: Missing "agent activity (active/queued, last command), plan/task progress, orchestrate loop state (enabled/disabled, iteration count, last cycle result), timestamps".
- **Required**: Expand `ConductorState` and `DetailsView` to include these fields. Ensure `polling.rs` extracts this from `session.json` and `iterations.jsonl`.

## 4. Logs Pane
- **Current Behavior**: Bottom-left pane shows a `subagent_tree`.
- **Gap**: Spec requires "streaming recent stdout/stderr/runtime logs" in the bottom-left pane.
- **Required**: Move `subagent_tree` or integrate it differently. Repurpose the bottom-left area for streaming logs (from `iterations.jsonl` or other sources).

## 5. Loop Engine Integration
- **Current Behavior**: Keybindings for Start/Pause/Resume only display status messages.
- **Gap**: No actual wiring to the orchestrate loop engine.
- **Required**: Implement logic to actually start/pause/resume orchestrate loops, likely by invoking the CLI or using the `orchestrate` library directly.

## 6. Live Reporting Sync
- **Current Behavior**: Polls every 500ms. Loads `session.json` and `iterations.jsonl`.
- **Gap**: `plan.md` changes aren't tracked/reloaded unless manually triggered. Agent activity is only partially reflected.
- **Required**: Improve polling to detect `plan.md` changes and reload the plan cache. Ensure agent state (from `session.json`) is fully utilized in the UI.

## 7. CLI Parity
- **Current Behavior**: Conductor is largely decoupled from CLI sessions unless they happen to target the same `tracks_dir`.
- **Gap**: Full parity with CLI-initiated sessions.
- **Required**: Ensure `ConductorPane` can attach to and monitor any session in `~/.maestro/orchestrate/`.
