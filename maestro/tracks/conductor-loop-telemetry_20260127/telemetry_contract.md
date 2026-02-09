# Telemetry Contract - Cockpit Conductor

This document defines the telemetry payloads and update cadence for the Conductor tab in Maestro Cockpit.

## 1. Data Model Equivalents

### Session Telemetry
Reflects the overall state of an orchestrate session.
- **Source**: `~/.maestro/orchestrate/<track_id>/session.json`
- **Fields**:
  - `session_id`: Unique ID for the session.
  - `track_id`: ID of the track being orchestrated.
  - `status`: `idle`, `running`, `paused`, `completed`, `failed`, `interrupted`.
  - `mode`: `planning`, `building`.
  - `current_iteration`: u64.
  - `current_task_id`: Option<String>.
  - `agent_config`: { `tool`, `model`, `dangerous`, `sandbox` }.
  - `rate_limit`: Option<RateLimitState>.

### Loop Telemetry
Reflects the current iteration details.
- **Source**: `~/.maestro/orchestrate/<track_id>/iterations.jsonl` (last entry)
- **Fields**:
  - `iteration`: u64.
  - `task_id`: String.
  - `status`: `running`, `completed`, `failed`, `skipped`.
  - `output`: String (stdout).
  - `error`: Option<String> (stderr).
  - `started_at`: Timestamp.
  - `completed_at`: Option<Timestamp>.

### Agent Activity
Reflects what the agent is currently doing.
- **Source**: Derived from `session.json` + `iterations.jsonl`.
- **Fields**:
  - `active_tool`: String.
  - `current_action`: String (e.g., "Thinking", "Executing Shell", "Reading File").
  - `last_command`: String (last executed shell command or tool call).
  - `since`: Timestamp.

## 2. Update Cadence
- **Poll Rate**: 500ms (standard Cockpit refresh).
- **Events**: File system watches (inotify/kqueue) should ideally trigger refresh for `session.json` and `iterations.jsonl`.

## 3. UI Mappings

### Tree View Nodes
- **Track Node**: Shows `[TRACK_ID]`, `[STATUS]`, `[ITERATION_COUNT]`.
- **Task Node**: Shows `[TASK_TITLE]`, `[STATUS]`, `[ACTIVE_INDICATOR]` if it's the `current_task_id`.

### Details Pane (Right)
- **Selection = Track**: Show session stats, agent config, loop controls, total progress.
- **Selection = Task**: Show task description, dependencies, history of iterations for this task.
- **Global**: Show active agent info and rate limit status.

### Logs Pane (Bottom-Left)
- **Streaming**: Tail the last N lines of `iterations.jsonl` output/error.
- **Format**:
  ```
  [14:05:01] [Agent] Thinking...
  [14:05:05] [Shell] ls -R
  [14:05:06] [Stdout] crates/ README.md ...
  ```

## 4. Discovery Protocol
1. Scan `~/.maestro/orchestrate/` for directories.
2. For each directory, check if `session.json` exists.
3. If it exists, it's an "Active/Known Session".
4. Add it to the Conductor's `tracks` list if not already present via `tracks.md`.
5. If it's NOT in `tracks.md`, mark it as a "CLI/External Session".
