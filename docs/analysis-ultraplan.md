# Ultraplan Feature — Code-Level Analysis

> **Scope:** Claude Code's Ultraplan feature — remote plan drafting via CCR (Claude Code on the web) with local CLI integration.

## Architecture Summary

Ultraplan launches a remote CCR session that drafts an advanced plan using Opus. The user can edit and approve the plan in a browser, and the approved plan is sent back to the local CLI for execution.

### Key Files

| File | Role |
|------|------|
| `src/commands/ultraplan.tsx` | Slash command entry point (`/ultraplan <prompt>`) |
| `src/utils/ultraplan/keyword.ts` | Keyword detection for auto-triggering |
| `src/utils/ultraplan/ccrSession.ts` | CCR session polling and plan extraction |
| `src/utils/ultraplan/prompt.txt` | System prompt for the remote agent |
| `src/components/UltraplanLaunchDialog.tsx` | Pre-launch confirmation dialog |
| `src/components/UltraplanChoiceDialog.tsx` | Post-plan choice dialog (execute vs dismiss) |
| `src/screens/REPL.tsx` | Main REPL — mounts dialogs based on AppState |

---

## Pillar 1: Ensuring the Model Launches with Proper Contents

### Trigger Paths

There are **two** ways to initiate Ultraplan:

1. **Slash command** — `/ultraplan <prompt>` handled by `src/commands/ultraplan.tsx`
2. **Keyword detection** — automatic trigger when `"ultraplan"` appears in user input

### Keyword Detection (`keyword.ts`)

`findUltraplanTriggerPositions()` performs sophisticated context-aware scanning of user input. It identifies occurrences of the keyword while **skipping** false positives in:

- Quoted ranges (single/double quotes)
- Backtick-delimited code spans
- Path-like contexts (e.g., `src/ultraplan/...`)
- Inputs ending with `?` suffix (treating as a question, not a command)
- Slash command inputs (already handled by the command path)

### Prompt Assembly

`buildUltraplanPrompt(blurb, seedPlan)` constructs the initial message sent to the remote session:

- **System prompt** — loaded from `prompt.txt`: *"Create a concise, execution-ready plan for the user's request."*
- **Seed plan** (optional) — included when upgrading from local plan mode, giving the remote model a starting point
- **User's blurb** — the actual request text

The assembled prompt is wrapped in `<system-reminder>` tags. This ensures the CCR browser UI hides the scaffolding from the user while the model sees the full instructional text.

### Model Selection

`getUltraplanModel()` reads the feature flag `tengu_ultraplan_model`, defaulting to **Opus** when the flag is absent.

---

## Pillar 2: Launching the Browser and Loading the UI

### Entry Point

`launchUltraplan()` is the **shared entry point** for all three trigger sources: slash command, keyword detection, and the plan-approval dialog.

### Launch Sequence

1. **Guard against duplicates** — Sets `ultraplanLaunching: true` in `AppState` synchronously before any async work begins.

2. **`launchDetached()`** is called, which:
   - Checks remote agent eligibility via `checkRemoteAgentEligibility()`
   - Calls `teleportToRemote()` with the assembled prompt, model, and `permissionMode: 'plan'`

3. **`teleportToRemote()`**:
   - Creates a remote CCR session via API
   - Bundles the local git repository context
   - Returns a session URL (e.g., `https://code.claude.com/sessions/<id>`)

4. **URL storage** — The session URL is stored in `AppState` as `ultraplanSessionUrl`. The browser is **not** opened automatically; the URL is displayed in the terminal for the user to navigate to manually.

### Confirmation Dialog (free-code variant)

In the `free-code` variant, `UltraplanLaunchDialog` is shown **before** launch. This is a bottom-area dialog using the `focusedInputDialog` pattern in `REPL.tsx`, requiring explicit user confirmation before proceeding.

### Remote Session Initialization

The CCR web UI loads the plan content because the `initialMessage` is sent as the **first user message** in the remote session. The model begins drafting immediately upon session creation.

---

## Pillar 3: Loading the Plan Content Made by the Model

### Polling Mechanism

After `teleportToRemote()` succeeds, `startDetachedPoll()` begins polling the remote session for results.

`pollForApprovedExitPlanMode()` in `ccrSession.ts` runs a polling loop with:

- **Interval:** 3 seconds
- **Timeout:** 30 minutes

### ExitPlanModeScanner

The scanner is a **stateful classifier** that ingests `SDKMessage[]` batches from `pollRemoteSessionEvents()`. It watches for `ExitPlanMode` tool_use blocks from the assistant, then inspects their `tool_result` to determine the outcome:

| `tool_result` condition | Interpretation |
|--------------------------|----------------|
| `is_error === false` | **Approved plan** — extracted via `extractApprovedPlan()` looking for the `"## Approved Plan:"` marker in the result content |
| `is_error === true` + `ULTRAPLAN_TELEPORT_SENTINEL` | **Teleport** — user clicked "teleport back to terminal" in the browser |
| `is_error === true` without sentinel | **Rejection** — normal iteration; user is still editing in the browser |

### Phase Tracking

`UltraplanPhase` is an enum with three states that drive the REPL's task pill display:

```
'running'      → Remote model is actively drafting
'needs_input'  → Waiting for user action in the browser
'plan_ready'   → Plan has been approved, ready for local execution
```

The `onPhaseChange` callback updates `RemoteAgentTask` state for real-time status display in the terminal.

---

## Pillar 4: Sending Feedback to the Model When User Sends It

### Plan Delivery to Local CLI

When the poll resolves, the outcome determines the next step based on `executionTarget`:

#### `executionTarget: 'local'` (teleport back)

1. `ultraplanPendingChoice` is set in `AppState`
2. `REPL.tsx` mounts `UltraplanChoiceDialog`, which displays the plan text (truncated to **2000 characters** for display)
3. User is presented with two choices:

   - **"Execute plan here"** — Creates a user message containing the plan text and injects it into the local conversation via `setMessages()`. The local model then executes the plan.
   - **"Dismiss"** — Discards the plan entirely.

4. **Both choices** trigger cleanup:
   - Mark the remote task as completed
   - Clear all ultraplan state from `AppState`
   - Archive the remote CCR session

#### `executionTarget: 'remote'`

The user chose to execute in the CCR browser directly. The CLI simply shows a notification with the session URL — no further local action is taken.

### Error Handling

`UltraplanPollError` provides typed error reasons for failure cases:

| Reason | Description |
|--------|-------------|
| `terminated` | Remote session was terminated unexpectedly |
| `timeout_pending` | Timeout while waiting for user input in browser |
| `timeout_no_plan` | Timeout with no plan produced |

### Stop Mechanism

`stopUltraplan()` performs a full teardown:

1. Archives the remote CCR session
2. Kills the local polling task
3. Clears all ultraplan-related URLs and state from `AppState`

---

## Data Flow Diagram

```
User Input
    │
    ├──→ /ultraplan <prompt>  (slash command)
    │         │
    └──→ keyword detection    (findUltraplanTriggerPositions)
              │
              ▼
     launchUltraplan()
              │
              ├── [free-code] UltraplanLaunchDialog → user confirms
              │
              ▼
     launchDetached()
              │
              ├── checkRemoteAgentEligibility()
              ├── buildUltraplanPrompt(blurb, seedPlan)
              └── teleportToRemote(prompt, model, permissionMode:'plan')
                        │
                        ▼
              CCR Session Created → URL stored in AppState
                        │
                        ▼
              startDetachedPoll()
                        │
                        ▼
              pollForApprovedExitPlanMode()  [3s interval, 30min timeout]
                        │
                        ├── ExitPlanModeScanner ingests SDKMessage[]
                        │
                        ▼
              ┌─────────────────────────────┐
              │   ExitPlanMode tool_result   │
              ├─────────────────────────────┤
              │ approved    → extract plan   │
              │ teleport    → pending choice │
              │ rejected    → continue poll  │
              │ error       → poll error     │
              └─────────────────────────────┘
                        │
                        ▼
              UltraplanChoiceDialog
                        │
              ┌─────────┴─────────┐
              ▼                   ▼
        "Execute here"       "Dismiss"
              │                   │
     inject plan into        discard plan
     local conversation           │
              │                   │
              └───────┬───────────┘
                      ▼
              cleanup: mark complete,
              clear state, archive session
```
