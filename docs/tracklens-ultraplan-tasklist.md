# TrackLens × Ultraplan — Blocking Task List

> Each phase blocks the next. Within a phase, tasks are ordered by dependency.
> Code suggestions are verbatim where confidence is high; marked `[SKETCH]` where they need adaptation to runtime context.

---

## Phase 1: Server Infrastructure

> **Blocks:** Phase 2 (UI), Phase 3 (Workflow Integration)
> **Files touched:** `src/leindex/src/tracklens/types.rs`, `src/leindex/src/tracklens/server.rs`, `src/leindex/src/tracklens/mod.rs`

---

### Task 1.1 — Add `TrackLensPhase` enum and `PhaseMetadata` to types.rs

**File:** `src/leindex/src/tracklens/types.rs`
**Blocks:** 1.2, 1.3, 1.4, all Phase 2, all Phase 5
**Rationale:** Every downstream consumer needs the phase enum before it can be wired into channels or endpoints.

Insert after the `AutonomyMode` enum (after line 167):

```rust
// ─── Review Phase ─────────────────────────────────────────────────────────────

/// Review lifecycle phase — reported to agents and displayed in UI.
/// Modeled after Ultraplan's UltraplanPhase but adapted for local review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackLensPhase {
    /// Server starting, browser opening
    Launching,
    /// Client connected, content loading
    Loading,
    /// User is reviewing/annotating (read-only)
    Reviewing,
    /// User is editing content inline
    Editing,
    /// User submitted a decision
    Decided,
}

/// Timing and interaction metadata attached to decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseMetadata {
    /// Total wall-clock time the review was open (milliseconds)
    pub review_duration_ms: u64,
    /// Number of inline edits made by the user
    pub edit_count: u32,
    /// Number of annotations created
    pub annotation_count: u32,
    /// Number of review iterations (reset cycles)
    pub iteration: u32,
}
```

Add two new optional fields to `TrackLensDecision` (after `autonomy_mode`, line 42):

```rust
    /// Inline-edited content returned by the user (None if no edits were made)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_content: Option<String>,
    /// Phase timing metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_metadata: Option<PhaseMetadata>,
```

Update the test `test_decision_serialization` to include the new fields:

```rust
    #[test]
    fn test_decision_serialization() {
        let decision = TrackLensDecision {
            behavior: DecisionBehavior::Allow,
            annotations: None,
            feedback: None,
            autonomy_mode: None,
            edited_content: None,
            phase_metadata: None,
        };

        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("\"behavior\":\"allow\""));

        // Verify optional fields are omitted when None
        assert!(!json.contains("edited_content"));
        assert!(!json.contains("phase_metadata"));
    }

    #[test]
    fn test_decision_with_edited_content() {
        let decision = TrackLensDecision {
            behavior: DecisionBehavior::Allow,
            annotations: None,
            feedback: None,
            autonomy_mode: None,
            edited_content: Some("# Edited Plan\n\nUser changed this.".to_string()),
            phase_metadata: Some(PhaseMetadata {
                review_duration_ms: 45000,
                edit_count: 3,
                annotation_count: 1,
                iteration: 1,
            }),
        };

        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("edited_content"));
        assert!(json.contains("review_duration_ms"));
    }
```

---

### Task 1.2 — Add phase tracking watch channel to ServerState

**File:** `src/leindex/src/tracklens/server.rs`
**Blocks:** 1.4, 1.5, all Phase 2, all Phase 5
**Rationale:** The phase channel is the backbone of real-time state communication between UI and agent.

Add to `ServerState` struct (after `deadline_rx`, line 70):

```rust
    /// Current review phase transmitter
    pub phase_tx: watch::Sender<TrackLensPhase>,
    /// Current review phase receiver
    pub phase_rx: watch::Receiver<TrackLensPhase>,
    /// Review iteration counter (incremented on each reset)
    pub iteration: Arc<std::sync::atomic::AtomicU32>,
```

Update imports at the top of server.rs (add to the `use super::types` line 27):

```rust
use super::types::{ReviewMode, TrackLensDecision, TrackLensPhase};
```

Update `TrackLensServer::new()` (inside the constructor, after the deadline channel creation at line 118):

```rust
        let (phase_tx, phase_rx) = watch::channel(TrackLensPhase::Launching);
```

Add these fields to the `ServerState` construction inside `new()` (after `deadline_rx`, within the `Arc::new(ServerState { ... })` block):

```rust
                phase_tx,
                phase_rx,
                iteration: Arc::new(std::sync::atomic::AtomicU32::new(0)),
```

Add methods to `impl TrackLensServer` (after `wait_for_decision`, line 287):

```rust
    /// Set the current review phase
    pub fn set_phase(&self, phase: TrackLensPhase) {
        let _ = self.state.phase_tx.send(phase);
    }

    /// Get the current review phase
    pub fn current_phase(&self) -> TrackLensPhase {
        *self.state.phase_rx.borrow()
    }

    /// Wait for a phase change (non-blocking poll for agents)
    pub async fn wait_for_phase_change(&self) -> anyhow::Result<TrackLensPhase> {
        let mut rx = self.state.phase_rx.clone();
        rx.changed()
            .await
            .map_err(|e| anyhow::anyhow!("Phase channel closed: {}", e))?;
        Ok(*rx.borrow())
    }
```

---

### Task 1.3 — Add phase and content-update HTTP endpoints

**File:** `src/leindex/src/tracklens/server.rs`
**Blocks:** Phase 2 (UI needs to POST phase changes), Phase 5 (agent polls phase)
**Rationale:** The HTTP surface is the bridge between the React UI and the Rust server.

Add routes to the router in `start()` (after `.route("/api/agents", get(get_agents))` at line 192):

```rust
            .route("/api/phase", get(get_phase))
            .route("/api/phase", post(set_phase))
            .route("/api/content", post(update_content))
```

Add the handler functions (after the `get_plan` handler, before the tests section):

```rust
/// Get current review phase
async fn get_phase(
    State(state): State<Arc<ServerState>>,
) -> Json<serde_json::Value> {
    let phase = *state.phase_rx.borrow();
    Json(serde_json::json!({ "phase": phase }))
}

/// Set review phase from client
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SetPhaseRequest {
    phase: TrackLensPhase,
}

async fn set_phase(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<SetPhaseRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .phase_tx
        .send(req.phase)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

/// Update review content in-flight (for seed plan refinement)
async fn update_content(
    State(state): State<Arc<ServerState>>,
    Json(content): Json<ReviewContent>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut current = state
        .content
        .write()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    *current = Some(content);
    Ok(StatusCode::OK)
}
```

---

### Task 1.4 — Multi-round review (reset endpoint)

**File:** `src/leindex/src/tracklens/server.rs`
**Blocks:** Phase 3 (workflows use deny→refine→re-present loop)
**Rationale:** Without reset, every deny requires tearing down and restarting the server. Ultraplan's CCR handles this implicitly (rejected tool_result → model tries again). We need explicit reset.

Add route in `start()`:

```rust
            .route("/api/reset", post(reset_review))
```

Add handler:

```rust
/// Reset review state for a new round without restarting the server.
/// Clears the decision, increments the iteration counter, and sets phase to Reviewing.
async fn reset_review(
    State(state): State<Arc<ServerState>>,
) -> Result<impl IntoResponse, StatusCode> {
    // Clear the decision by sending None
    state
        .decision_tx
        .send(None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Increment iteration counter
    state
        .iteration
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    // Reset phase to Reviewing
    state
        .phase_tx
        .send(TrackLensPhase::Reviewing)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}
```

Add a `reset_for_resubmit` method to `impl TrackLensServer`:

```rust
    /// Reset server state for a new review round.
    /// Used by the agent after processing a denial to re-present updated content.
    pub fn reset_for_resubmit(&self, new_content: ReviewContent) -> anyhow::Result<()> {
        // Update content
        self.set_content(new_content)?;
        // Clear decision
        let _ = self.state.decision_tx.send(None);
        // Increment iteration
        self.state.iteration.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // Reset phase
        self.set_phase(TrackLensPhase::Reviewing);
        Ok(())
    }

    /// Get current iteration number
    pub fn iteration(&self) -> u32 {
        self.state.iteration.load(std::sync::atomic::Ordering::SeqCst)
    }
```

---

### Task 1.5 — Graceful shutdown

**File:** `src/leindex/src/tracklens/server.rs`
**Blocks:** Phase 3 (workflows need to clean up after review completes)
**Rationale:** Currently the server is spawned and never explicitly shut down. Ultraplan archives the remote session; we need a local equivalent.

Add a shutdown token to `ServerState`:

```rust
    /// Shutdown signal
    pub shutdown_tx: watch::Sender<bool>,
    pub shutdown_rx: watch::Receiver<bool>,
```

Initialize in `new()`:

```rust
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
```

Change the server spawn in `start()` to respect the shutdown signal. Replace the current spawn block (lines 230-235):

```rust
        // Spawn server in background with graceful shutdown
        let mut shutdown_rx_clone = self.state.shutdown_rx.clone();
        tokio::spawn(async move {
            let graceful = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    loop {
                        if shutdown_rx_clone.changed().await.is_err() {
                            break;
                        }
                        if *shutdown_rx_clone.borrow() {
                            break;
                        }
                    }
                });
            if let Err(e) = graceful.await {
                eprintln!("Server error: {}", e);
            }
        });
```

Add shutdown method:

```rust
    /// Trigger graceful server shutdown
    pub fn shutdown(&self) {
        let _ = self.state.shutdown_tx.send(true);
    }
```

Add shutdown endpoint and route:

```rust
            .route("/api/shutdown", post(shutdown_server))
```

```rust
async fn shutdown_server(
    State(state): State<Arc<ServerState>>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .shutdown_tx
        .send(true)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}
```

---

### Task 1.6 — Update mod.rs re-exports

**File:** `src/leindex/src/tracklens/mod.rs`
**Blocks:** All consumers of the tracklens module

Replace line 16-18:

```rust
pub use server::{ReviewContent, ReviewMetadata, ServerConfig, TrackLensServer};
pub use types::*;
pub use walkthrough::{WalkthroughConfig, WalkthroughGenerator};
```

No change needed — `pub use types::*` already re-exports everything from types.rs, including the new `TrackLensPhase` and `PhaseMetadata`.

---

## Phase 2: UI Enhancements

> **Blocks:** Phase 3 (workflows need editing support to function)
> **Files touched:** `packages/tracklens-editor/src/App.tsx`, `packages/tracklens-editor/src/main.tsx`, build config

---

### Task 2.1 — Add inline editing toggle to React app

**File:** `packages/tracklens-editor/src/App.tsx`
**Blocks:** 2.3, Phase 3 (edited_content flow)
**Rationale:** The core differentiator from current TrackLens — users can edit, not just annotate.

`[SKETCH]` — The minified bundle in `crates/cli/dist/tracklens-editor.html` needs to be rebuilt from `packages/tracklens-editor/src/`. The changes are:

1. Add state for edit mode and edited content:
```tsx
const [editMode, setEditMode] = useState(false);
const [editedMarkdown, setEditedMarkdown] = useState<string | null>(null);
```

2. Add an Edit toggle button in the header (next to Export):
```tsx
<button
  onClick={() => {
    setEditMode(!editMode);
    if (!editMode && !editedMarkdown) {
      setEditedMarkdown(plan); // Initialize edited copy from original
    }
  }}
  className={`px-3 py-1.5 text-sm rounded-lg transition-opacity ${
    editMode
      ? 'bg-yellow-600 text-white'
      : 'bg-muted text-foreground hover:bg-muted/80'
  }`}
>
  {editMode ? 'Preview' : 'Edit'}
</button>
```

3. Conditionally render either the annotation view or a textarea:
```tsx
{editMode ? (
  <textarea
    value={editedMarkdown || plan}
    onChange={(e) => setEditedMarkdown(e.target.value)}
    className="w-full h-full p-4 bg-background text-foreground font-mono text-sm resize-none focus:outline-none"
    spellCheck={false}
  />
) : (
  <AnnotationEditorView
    ref={editorRef}
    blocks={blocks}
    markdown={editedMarkdown || plan}
    /* ... existing props ... */
  />
)}
```

4. Include `edited_content` in the Approve decision POST:
```tsx
const approvePayload = {
  approved: true,
  feedback: serializeAnnotations(blocks, annotations),
  edited_content: editedMarkdown !== plan ? editedMarkdown : undefined,
  /* ... existing fields ... */
};
```

5. POST phase change to server when toggling edit mode:
```tsx
useEffect(() => {
  fetch('/api/phase', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ phase: editMode ? 'editing' : 'reviewing' }),
  }).catch(() => {});
}, [editMode]);
```

---

### Task 2.2 — Add phase indicator in UI header

**File:** `packages/tracklens-editor/src/App.tsx`
**Blocks:** Phase 5 (agent needs to see phase)
**Rationale:** Visual feedback that the server is tracking user activity.

`[SKETCH]`:

```tsx
const [phase, setPhase] = useState<string>('reviewing');

// Poll phase on mount and after changes
useEffect(() => {
  const interval = setInterval(() => {
    fetch('/api/phase')
      .then(r => r.json())
      .then(data => setPhase(data.phase))
      .catch(() => {});
  }, 2000);
  return () => clearInterval(interval);
}, []);

// In header JSX:
<span className="px-2 py-1 text-xs rounded bg-muted text-muted-foreground">
  {phase === 'reviewing' ? '👁 Reviewing' :
   phase === 'editing' ? '✏️ Editing' :
   phase === 'decided' ? '✓ Decided' : phase}
</span>
```

---

### Task 2.3 — Add keyboard shortcuts

**File:** `packages/tracklens-editor/src/App.tsx`
**Blocks:** Nothing (polish task)

```tsx
useEffect(() => {
  const handler = (e: KeyboardEvent) => {
    if (e.ctrlKey || e.metaKey) {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleApprove();
      } else if (e.key === 'Enter' && e.shiftKey) {
        e.preventDefault();
        handleDeny();
      } else if (e.key === 'e') {
        e.preventDefault();
        setEditMode(prev => !prev);
      }
    }
  };
  window.addEventListener('keydown', handler);
  return () => window.removeEventListener('keydown', handler);
}, [handleApprove, handleDeny]);
```

Add shortcut hints in the footer:

```tsx
<div className="text-xs text-muted-foreground">
  ⌘Enter Approve · ⌘⇧Enter Deny · ⌘E Edit
</div>
```

---

### Task 2.4 — Rebuild bundle

**Steps:**
1. `cd packages/tracklens-editor && npm run build`
2. Copy output to `crates/cli/dist/tracklens-editor.html`
3. Verify `find_bundle_dir()` resolves the new bundle
4. Run `cargo test -p leindex-core --lib tracklens` to verify server integration tests still pass

---

## Phase 3: Workflow Integration

> **Blocks:** Phase 4 (keyword detection needs workflows to exist)
> **Files touched:** `amp-cli/skills/maestro/SKILL.md`, `claude-code/skills/maestro/SKILL.md`, `gemini-cli/skills/maestro/SKILL.md`, `pi-maestro/src/commands/newTrack.ts`, `pi-maestro/src/commands/setup.ts`, `pi-maestro/src/commands/implement.ts`, `pi-maestro/src/commands/orchestrate.ts`, `pi-maestro/src/tracklens/extension/tools.ts`

---

### Task 3.0 — Add TrackLens Review Protocol to maestro skill (LLM-facing)

**Files:** `amp-cli/skills/maestro/SKILL.md`, `claude-code/skills/maestro/SKILL.md`, `gemini-cli/skills/maestro/SKILL.md`
**Blocks:** 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, and all of Phase 4
**Rationale:** Keyword detection (Phase 4) only handles **user-initiated** triggers — the user types "tracklens" or "review this." Workflow hooks (Tasks 3.1–3.4) only fire when a maestro command is explicitly invoked. Neither covers the case where the LLM generates a reviewable document in freeform conversation and should **autonomously** invoke TrackLens. A skill section solves this by teaching the LLM *judgment* about when to call `tracklens_review` versus `tracklens_walkthrough`, persisting across the entire conversation regardless of how the work was initiated.

**Status: ✅ DONE** — Added to all three skill copies. Content:

```markdown
## TrackLens Review Protocol

You MUST call `tracklens_review` in these situations:
- After drafting or substantially revising any `spec.md` or `plan.md`
- After generating any markdown document the user will need to approve or act on
- When the user asks you to "review", "check", or "look over" a document you produced
- After `maestro:setup` generates product/tech-stack/workflow documents

You MUST call `tracklens_walkthrough` after completing all tasks in a track or when the user wants a review of the completed implementation as a whole.

You MUST NOT call `tracklens_review`:
- For trivial edits (typo fixes, formatting, single-line changes)
- When the user explicitly says to skip review
- For intermediate drafts the user hasn't asked to see yet
- For files you are only reading, not producing

If the user asks for review but there is no clear reviewable artifact yet:
- Identify the exact file, markdown artifact, or track output first
- Do not guess which document to open in TrackLens
- If the request is for implementation output rather than a document, prefer `tracklens_walkthrough`

When a review is **denied**:
1. Read every annotation — pay attention to severity (`ERROR` > `WARNING` > `INFO`)
2. Address each annotation in severity order
3. If `edited_content` is returned, use the user's edited version as your new baseline
4. Re-call `tracklens_review` with the updated content
5. Do NOT mark work as complete until review is approved
6. After 3 consecutive denials on the same document, ask the user what they want changed instead of guessing

When a walkthrough is **denied**:
1. Convert the denial into remediation work
2. Complete the remediation work before re-running `tracklens_walkthrough`
3. Do NOT mark the track complete until the walkthrough is approved

When a review is **approved**:
- If `edited_content` is present, write that version to disk (the user refined your draft)
- If no `edited_content`, your draft was accepted as-is
- Proceed to the next workflow step
```

**Why a skill and not just workflow hooks:**

| Trigger scenario | Keyword detection | Workflow hook | Skill |
|---|---|---|---|
| User types "tracklens" | ✅ | ❌ | ❌ |
| `/maestro:newTrack` → agent drafts spec | ❌ | ✅ | ✅ |
| "Write me a spec for auth" (no command) | ❌ | ❌ | **✅** |
| Agent finishes implementing (general prompt) | ❌ | ❌ | **✅** (`tracklens_walkthrough`) |
| Agent generates any `.md` artifact | ❌ | ❌ | **✅** |

All three mechanisms are complementary:
- **Skill** = autonomous invocation (LLM decides)
- **Workflow hooks** = mechanical invocation (command triggers)
- **Keyword detection** = user-initiated invocation (human triggers)

---

### Task 3.1 — Integrate TrackLens into `maestro:newTrack`

**File:** `pi-maestro/src/commands/newTrack.ts`
**Blocks:** 3.3 (implement depends on newTrack having reviewed specs)
**Rationale:** The highest-value integration point. Every spec and plan should be reviewed before implementation starts.

In the `before_agent_start` handler that injects workflow instructions, append TrackLens review steps to the injected workflow prompt:

```typescript
// [SKETCH] — Add to the workflow instructions string injected into the agent context
const tracklensWorkflowInstructions = `
## TrackLens Review Checkpoints

After drafting spec.md:
1. Call the tracklens_review tool with the spec.md content and documentType "spec.md"
2. If the review is denied, incorporate the annotations and feedback, then re-draft
3. Repeat until approved or 3 iterations reached

After drafting plan.md:
1. Call the tracklens_review tool with the plan.md content and documentType "plan.md"
2. If the review is denied, incorporate the annotations and feedback, then re-draft
3. Repeat until approved or 3 iterations reached

If the user's edited_content is returned in an approved review, use that version as the final document instead of your draft.
`;
```

---

### Task 3.2 — Integrate TrackLens into `maestro:setup`

**File:** `pi-maestro/src/commands/setup.ts`
**Blocks:** Nothing (independent from other workflow integrations)
**Rationale:** Setup generates foundational docs — catching issues early prevents downstream waste.

After `initializeMaestroProject()` generates all documents, add a combined review checkpoint:

```typescript
// [SKETCH] — After all setup files are generated
const setupDocs = [
  { name: 'product.md', content: productMd },
  { name: 'tech-stack.md', content: techStackMd },
  { name: 'workflow.md', content: workflowMd },
].map(d => `## ${d.name}\n\n${d.content}`).join('\n\n---\n\n');

// Trigger TrackLens review via tool call
pi.sendMessage({
  customType: "maestro-setup-review",
  content: `Setup documents generated. Requesting TrackLens review...`,
  display: true,
}, { triggerTurn: true });

// In before_agent_start, inject instruction to call tracklens_review
```

---

### Task 3.3 — Strengthen `maestro:implement` integration

**File:** `pi-maestro/src/commands/implement.ts`
**Blocks:** 3.4 (orchestrate depends on implement walkthrough flow)
**Rationale:** Currently implement only checks `isTrackLensEnabled()` as an optional gate. Make it the default path.

Locate the workflow injection in `before_agent_start` and add mandatory walkthrough instructions:

```typescript
// [SKETCH] — Append to the implement workflow instructions
const walkthroughInstructions = `
## Post-Implementation Walkthrough (MANDATORY)

After completing all tasks in the plan:
1. Call tracklens_walkthrough with the trackId
2. The walkthrough will be generated automatically and presented for review
3. If denied:
   - Annotations will be converted to remediation tasks
   - Address each remediation task
   - The walkthrough will be regenerated and re-presented
4. Do NOT mark the track as completed until the walkthrough is approved
`;
```

Remove the `isTrackLensEnabled()` guard for walkthrough — it should always run. Keep the guard only for intermediate code-review checkpoints.

---

### Task 3.4 — Integrate TrackLens into `maestro:orchestrate`

**File:** `pi-maestro/src/commands/orchestrate.ts`
**Blocks:** Nothing
**Rationale:** The orchestrator currently only shows notifications. It should block on walkthrough approval.

Replace the notification-only pattern (around line 80) with an actual tool invocation:

```typescript
// Replace:
//   ctx.ui.notify(`Requesting TrackLens walkthrough for ${subtrackId}...`, "info");
// With:

// [SKETCH] — In the sub-track completion handler
if (result === "completed") {
  completedCount++;

  // Trigger walkthrough review as part of orchestration
  pi.sendMessage({
    customType: "maestro-orchestrate-walkthrough",
    content: `Sub-track ${subtrackId} completed. Generating walkthrough for review...`,
    metadata: { trackId: subtrackId, action: "walkthrough" },
    display: true,
  }, { triggerTurn: true });
}
```

At master track completion, trigger aggregate review:

```typescript
// [SKETCH] — After all sub-tracks complete
if (failedCount === 0) {
  // Generate aggregate walkthrough for master track
  pi.sendMessage({
    customType: "maestro-orchestrate-master-walkthrough",
    content: `All sub-tracks complete. Generating master walkthrough for ${trackId}...`,
    metadata: { trackId, action: "master-walkthrough" },
    display: true,
  }, { triggerTurn: true });
}
```

---

### Task 3.5 — Add `tracklens_code_review` tool

**File:** `pi-maestro/src/tracklens/extension/tools.ts`
**Blocks:** Nothing (additive feature)
**Rationale:** Code review is already supported by the server and CLI, but not as a pi-maestro tool.

Add after the `tracklens_walkthrough` tool registration:

```typescript
  pi.registerTool({
    name: "tracklens_code_review",
    label: "TrackLens Code Review",
    description: `
      Request TrackLens code review for git changes.

      Use this tool to present a git diff for user review:
      - After making significant code changes
      - Before committing to verify changes look correct
      - When the user requests a code review

      The diff will be presented in a dedicated code review UI.
    `.trim(),
    parameters: {
      type: "object",
      properties: {
        gitRef: {
          type: "string",
          description: "Git ref to diff against (default: HEAD)",
          default: "HEAD",
        },
        files: {
          type: "array",
          items: { type: "string" },
          description: "Specific files to include in the diff (optional, defaults to all)",
        },
      },
    },

    async execute(_toolCallId, params, _signal, _onUpdate, ctx) {
      const { gitRef = "HEAD", files } = params as {
        gitRef?: string;
        files?: string[];
      };

      // Generate diff
      const { execSync } = await import("child_process");
      let diff: string;
      try {
        const fileArgs = files ? ["--", ...files] : [];
        const args = ["diff", gitRef, ...fileArgs];
        diff = execSync(`git ${args.join(" ")}`, {
          cwd: ctx.cwd,
          encoding: "utf-8",
        });
      } catch (error) {
        return {
          content: [{ type: "text", text: `Git diff failed: ${error}` }],
          details: { approved: false },
        };
      }

      if (!diff.trim()) {
        return {
          content: [{ type: "text", text: "No changes found in diff." }],
          details: { approved: true },
        };
      }

      // Launch TrackLens in code-review mode
      let startTrackLensServer: any;
      let htmlContent: string | null = null;

      try {
        const tracklensServer = await import("@maestro/tracklens-server");
        startTrackLensServer = tracklensServer.startTrackLensServer;

        const { existsSync: exists, readFileSync: read } = await import("fs");
        const { resolve } = await import("path");
        const htmlPaths = [
          resolve(ctx.cwd, "apps/tracklens-opencode/tracklens.html"),
          resolve(ctx.cwd, "dist/tracklens-editor.html"),
        ];
        for (const htmlPath of htmlPaths) {
          if (exists(htmlPath)) {
            htmlContent = read(htmlPath, "utf-8");
            break;
          }
        }
      } catch {
        return {
          content: [{ type: "text", text: `Code review diff:\n\`\`\`diff\n${diff}\n\`\`\`\nTrackLens UI not available. Please review manually.` }],
          details: { approved: false, manualReview: true },
        };
      }

      try {
        const server = await startTrackLensServer({
          plan: diff,
          origin: "pi-maestro",
          htmlContent,
          mode: "code-review",
        });

        const result = await server.waitForDecision();
        server.stop();

        return {
          content: [{
            type: "text",
            text: result.approved
              ? "Code review approved."
              : `Code review denied. Feedback:\n${result.feedback || "No feedback provided."}`,
          }],
          details: {
            approved: result.approved,
            annotations: result.annotations,
          },
        };
      } catch (error) {
        return {
          content: [{ type: "text", text: `TrackLens error: ${error}. Please review manually.` }],
          details: { approved: false, manualReview: true },
        };
      }
    },
  });
```

---

### Task 3.6 — Add `seedContent` parameter to `tracklens_review`

**File:** `pi-maestro/src/tracklens/extension/tools.ts`
**Blocks:** Task 4.3 (seed plan support via keyword)
**Rationale:** Ultraplan's `buildUltraplanPrompt(blurb, seedPlan)` prepends a draft for refinement. TrackLens needs the same.

Add to the `tracklens_review` tool's parameters schema (add after `filePath`):

```typescript
        seedContent: {
          type: "string",
          description: "Optional seed/draft content to pre-populate the editor. User can edit this and approve the edited version.",
        },
```

In the execute function, if `seedContent` is provided, use it as the initial editable content and pass a flag to the server:

```typescript
      // If seedContent is provided, the review content includes an editing hint
      const reviewMarkdown = seedContent
        ? `<!-- tracklens:editable -->\n${seedContent}`
        : markdown;
```

---

## Phase 4: Keyword Detection & Auto-Trigger

> **Blocks:** Phase 5 (feedback loop needs trigger paths)
> **Files touched:** New file `pi-maestro/src/tracklens/keyword.ts`, `pi-maestro/src/index.ts`

---

### Task 4.1 — Port keyword detection from Ultraplan

**New file:** `pi-maestro/src/tracklens/keyword.ts`
**Blocks:** 4.2
**Rationale:** Direct port of Ultraplan's battle-tested keyword detection with TrackLens-specific keywords.

```typescript
/**
 * TrackLens Keyword Detection
 *
 * Ported from Claude Code's Ultraplan keyword.ts — same delimiter-aware,
 * path-aware, question-aware filtering logic.
 *
 * @packageDocumentation
 */

type TriggerPosition = { word: string; start: number; end: number };

const OPEN_TO_CLOSE: Record<string, string> = {
  '`': '`',
  '"': '"',
  '<': '>',
  '{': '}',
  '[': ']',
  '(': ')',
  "'": "'",
};

/**
 * Find keyword positions, skipping occurrences inside delimiters,
 * path-like contexts, or question contexts.
 *
 * Adapted from Ultraplan's findKeywordTriggerPositions.
 */
function findKeywordTriggerPositions(
  text: string,
  keyword: string,
): TriggerPosition[] {
  const re = new RegExp(keyword, 'i');
  if (!re.test(text)) return [];
  if (text.startsWith('/')) return []; // Slash command — don't trigger

  const quotedRanges: Array<{ start: number; end: number }> = [];
  let openQuote: string | null = null;
  let openAt = 0;
  const isWord = (ch: string | undefined) => !!ch && /[\p{L}\p{N}_]/u.test(ch);

  for (let i = 0; i < text.length; i++) {
    const ch = text[i]!;
    if (openQuote) {
      if (openQuote === '[' && ch === '[') { openAt = i; continue; }
      if (ch !== OPEN_TO_CLOSE[openQuote]) continue;
      if (openQuote === "'" && isWord(text[i + 1])) continue;
      quotedRanges.push({ start: openAt, end: i + 1 });
      openQuote = null;
    } else if (
      (ch === '<' && i + 1 < text.length && /[a-zA-Z/]/.test(text[i + 1]!)) ||
      (ch === "'" && !isWord(text[i - 1])) ||
      (ch !== '<' && ch !== "'" && ch in OPEN_TO_CLOSE)
    ) {
      openQuote = ch;
      openAt = i;
    }
  }

  const positions: TriggerPosition[] = [];
  const wordRe = new RegExp(`\\b${keyword}\\b`, 'gi');
  const matches = text.matchAll(wordRe);
  for (const match of matches) {
    if (match.index === undefined) continue;
    const start = match.index;
    const end = start + match[0].length;
    if (quotedRanges.some(r => start >= r.start && start < r.end)) continue;
    const before = text[start - 1];
    const after = text[end];
    if (before === '/' || before === '\\' || before === '-') continue;
    if (after === '/' || after === '\\' || after === '-' || after === '?') continue;
    if (after === '.' && isWord(text[end + 1])) continue;
    positions.push({ word: match[0], start, end });
  }
  return positions;
}

export function findTrackLensTriggerPositions(text: string): TriggerPosition[] {
  return findKeywordTriggerPositions(text, 'tracklens');
}

export function hasTrackLensKeyword(text: string): boolean {
  return findTrackLensTriggerPositions(text).length > 0;
}

/**
 * Check if text contains a "review this" trigger.
 * Only matches standalone "review this" (not "review this code" etc. which would
 * be too aggressive). The bare phrase acts like Ultraplan's keyword trigger.
 */
export function hasReviewTrigger(text: string): boolean {
  // "review this" at end of sentence or as standalone phrase
  return /\breview\s+this\b[\s.,!]*$/i.test(text.trim());
}

/**
 * Replace the first triggerable "tracklens" keyword so the forwarded
 * prompt stays grammatical.
 */
export function replaceTrackLensKeyword(text: string): string {
  const [trigger] = findTrackLensTriggerPositions(text);
  if (!trigger) return text;
  const before = text.slice(0, trigger.start);
  const after = text.slice(trigger.end);
  if (!(before + after).trim()) return '';
  return (before + after).trim();
}
```

---

### Task 4.2 — Wire keyword detection into pi-maestro message processing

**File:** `pi-maestro/src/index.ts` (or wherever `before_agent_start` is registered globally)
**Blocks:** Nothing
**Rationale:** This is the auto-trigger path — when a user says "tracklens" anywhere in their message, we intercept.

`[SKETCH]` — The exact integration depends on pi-maestro's message hook architecture:

```typescript
import { hasTrackLensKeyword, hasReviewTrigger } from "./tracklens/keyword.js";

// In the global message processing hook:
pi.on("before_send_message", async (event) => {
  const userText = event.text;

  if (hasTrackLensKeyword(userText) || hasReviewTrigger(userText)) {
    // Check if there's a recent artifact to review. Keep this bounded so
    // stale documents do not auto-open TrackLens minutes later.
    const lastGeneratedDoc = getLastGeneratedDocument({ maxAgeMs: 10 * 60 * 1000 }); // [SKETCH]

    if (lastGeneratedDoc) {
      // Auto-invoke TrackLens review
      event.preventDefault(); // Don't send to model
      const toolName = lastGeneratedDoc.kind === "walkthrough"
        ? "tracklens_walkthrough"
        : "tracklens_review";

      if (!event.metadata?.tracklensAutoTriggered) {
        event.metadata = { ...event.metadata, tracklensAutoTriggered: true };
        pi.invokeTool(
          toolName,
          toolName === "tracklens_walkthrough"
            ? { trackId: lastGeneratedDoc.trackId }
            : {
                markdown: lastGeneratedDoc.content,
                documentType: lastGeneratedDoc.type,
                trackId: lastGeneratedDoc.trackId,
              },
        );
      }
    }
    // If no recent artifact exists, let the message through so the model can
    // ask which document or track output the user wants reviewed.
  }
});
```

---

### Task 4.3 — Seed plan support in tool invocation

**File:** `pi-maestro/src/tracklens/extension/tools.ts`
**Blocks:** Nothing
**Rationale:** Covered in Task 3.6. This task is about the end-to-end flow — when seed content is provided, the UI should start in edit mode.

On the server side, detect the `<!-- tracklens:editable -->` marker and set the initial phase to `Editing`:

```rust
// [SKETCH] — In server.rs set_content method, after setting content:
if content.content.starts_with("<!-- tracklens:editable -->") {
    let _ = self.state.phase_tx.send(TrackLensPhase::Editing);
}
```

On the UI side, check for the marker on mount:

```tsx
// [SKETCH] — In App.tsx useEffect for /api/plan fetch
useEffect(() => {
  fetch('/api/plan').then(r => r.json()).then(data => {
    let planContent = data.plan;
    if (planContent.startsWith('<!-- tracklens:editable -->')) {
      planContent = planContent.replace('<!-- tracklens:editable -->\n', '');
      setEditMode(true);
      setEditedMarkdown(planContent);
    }
    setPlan(planContent);
    setBlocks(parseBlocks(planContent));
  });
}, []);
```

---

## Phase 5: Agent-Side Feedback Loop

> **Depends on:** Phase 1, Phase 3
> **Files touched:** `pi-maestro/src/tracklens/extension/tools.ts`, new files

---

### Task 5.1 — Structured feedback formatting on denial

**File:** `pi-maestro/src/tracklens/extension/tools.ts`
**Blocks:** 5.3 (history needs structured feedback)
**Rationale:** When the agent receives a denial, it needs structured data it can act on, not a blob of JSON.

In the `tracklens_review` tool's execute function, after receiving a denial result, format annotations as structured feedback:

```typescript
      // After: const result = await server.waitForDecision();
      if (!result.approved) {
        const structuredFeedback = formatDenialForAgent(result);
        return {
          content: [{ type: "text", text: structuredFeedback }],
          details: {
            approved: false,
            annotationCount: result.annotations?.length || 0,
            editedContent: result.edited_content,
          },
        };
      }
```

Add the formatting function:

```typescript
function formatDenialForAgent(result: any): string {
  const parts: string[] = ['# Review Denied\n'];

  if (result.feedback) {
    parts.push(`## General Feedback\n\n${result.feedback}\n`);
  }

  if (result.annotations && result.annotations.length > 0) {
    parts.push(`## Annotations (${result.annotations.length})\n`);

    for (const ann of result.annotations) {
      const severity = ann.content?.severity?.toUpperCase() || 'INFO';
      const line = ann.selection?.start?.line || '?';
      const text = ann.selection?.text || '';
      const comment = ann.content?.comment || '';

      parts.push(`### [${severity}] Line ${line}`);
      if (text) parts.push(`> ${text.split('\n').join('\n> ')}`);
      parts.push(`\n${comment}\n`);
    }
  }

  parts.push('\n---\nAddress the above feedback and call tracklens_review again with the updated content.');
  return parts.join('\n');
}
```

---

### Task 5.2 — Phase reporting to agent

**New file:** `pi-maestro/src/tracklens/phaseReporter.ts`
**Blocks:** Nothing (polish)
**Rationale:** Agents benefit from knowing the user is actively reviewing (prevents timeouts, gives status context).

```typescript
/**
 * TrackLens Phase Reporter
 *
 * Polls the TrackLens server for phase changes and reports them
 * to the agent context, similar to Ultraplan's onPhaseChange callback.
 *
 * @packageDocumentation
 */

export interface PhaseReporterOptions {
  serverUrl: string;
  pollIntervalMs?: number;
  onPhaseChange: (phase: string) => void;
  signal?: AbortSignal;
}

export async function startPhaseReporter(opts: PhaseReporterOptions): Promise<void> {
  const { serverUrl, pollIntervalMs = 3000, onPhaseChange, signal } = opts;
  let lastPhase = '';

  while (!signal?.aborted) {
    try {
      const resp = await fetch(`${serverUrl}/api/phase`, { signal });
      if (resp.ok) {
        const data = await resp.json();
        if (data.phase !== lastPhase) {
          lastPhase = data.phase;
          onPhaseChange(data.phase);
        }
      }
    } catch {
      // Server may not be running yet or was shut down
      if (signal?.aborted) break;
    }

    await new Promise<void>((resolve) => {
      const timer = setTimeout(resolve, pollIntervalMs);
      signal?.addEventListener('abort', () => { clearTimeout(timer); resolve(); }, { once: true });
    });
  }
}
```

Usage in the tool execute function:

```typescript
// [SKETCH] — After server starts, before waiting for decision
const phaseAbort = new AbortController();
startPhaseReporter({
  serverUrl: server.url,
  onPhaseChange: (phase) => {
    onUpdate?.(`TrackLens: User is ${phase}`);
  },
  signal: phaseAbort.signal,
});

// After decision received:
phaseAbort.abort();
```

---

### Task 5.3 — Review history persistence

**New file:** `pi-maestro/src/tracklens/history.ts`
**Blocks:** Nothing (additive feature)
**Rationale:** Agents and users benefit from seeing prior review feedback when re-drafting.

```typescript
/**
 * TrackLens Review History
 *
 * Persists review decisions and annotations per track/document.
 *
 * @packageDocumentation
 */

import { readFile, writeFile, mkdir } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";

export interface ReviewHistoryEntry {
  timestamp: string;
  documentType: string;
  approved: boolean;
  annotationCount: number;
  feedback?: string;
  editedContent?: boolean; // true if user edited, not the content itself (too large)
  reviewDurationMs?: number;
  iteration: number;
}

export interface ReviewHistory {
  trackId: string;
  entries: ReviewHistoryEntry[];
}

const HISTORY_FILENAME = "review-history.json";

export async function loadReviewHistory(
  trackDir: string,
  trackId: string,
): Promise<ReviewHistory> {
  const historyPath = join(trackDir, HISTORY_FILENAME);
  if (!existsSync(historyPath)) {
    return { trackId, entries: [] };
  }
  const raw = await readFile(historyPath, "utf-8");
  return JSON.parse(raw);
}

export async function appendReviewEntry(
  trackDir: string,
  trackId: string,
  entry: ReviewHistoryEntry,
): Promise<void> {
  const history = await loadReviewHistory(trackDir, trackId);
  history.entries.push(entry);
  await mkdir(trackDir, { recursive: true });
  await writeFile(
    join(trackDir, HISTORY_FILENAME),
    JSON.stringify(history, null, 2),
    "utf-8",
  );
}

/**
 * Format review history as context for the agent
 */
export function formatHistoryForAgent(history: ReviewHistory): string {
  if (history.entries.length === 0) return '';

  const lines = [`## Prior Reviews for ${history.trackId}\n`];
  for (const entry of history.entries.slice(-5)) { // Last 5 entries
    const status = entry.approved ? '✓ Approved' : '✗ Denied';
    const date = new Date(entry.timestamp).toLocaleDateString();
    lines.push(`- **${date}** ${entry.documentType}: ${status} (${entry.annotationCount} annotations)`);
    if (entry.feedback) {
      lines.push(`  > ${entry.feedback.slice(0, 200)}`);
    }
  }
  return lines.join('\n');
}
```

Wire into `tracklens_review` tool to auto-persist:

```typescript
// [SKETCH] — After receiving decision in tracklens_review execute()
import { appendReviewEntry } from "../history.js";

// After result is received:
if (trackId) {
  const trackDir = resolve(root, "maestro/tracks", trackId);
  await appendReviewEntry(trackDir, trackId, {
    timestamp: new Date().toISOString(),
    documentType,
    approved: result.approved,
    annotationCount: result.annotations?.length || 0,
    feedback: result.feedback,
    editedContent: !!result.edited_content,
    reviewDurationMs: result.phase_metadata?.review_duration_ms,
    iteration: result.phase_metadata?.iteration || 0,
  });
}
```

---

## Dependency Graph

```
Phase 1 (Server)
  ├── 1.1 Types ──────────┐
  ├── 1.2 Phase channel ──┤
  ├── 1.3 HTTP endpoints ─┤── Phase 2 (UI)
  ├── 1.4 Reset endpoint ─┤     ├── 2.1 Edit mode
  ├── 1.5 Shutdown ────────┤     ├── 2.2 Phase indicator
  └── 1.6 Re-exports ─────┘     ├── 2.3 Keyboard shortcuts
                                 └── 2.4 Build bundle
                                       │
                                 Phase 3 (Workflows)
                                   ├── 3.0 Skill (LLM-facing) ✅ DONE
                                   ├── 3.1 newTrack
                                   ├── 3.2 setup
                                   ├── 3.3 implement
                                   ├── 3.4 orchestrate
                                   ├── 3.5 code-review tool
                                   └── 3.6 seedContent param
                                         │
                                   Phase 4 (Keywords)
                                     ├── 4.1 Detection module
                                     ├── 4.2 Message hook
                                     └── 4.3 Seed plan flow
                                           │
                                   Phase 5 (Feedback)
                                     ├── 5.1 Structured denial
                                     ├── 5.2 Phase reporter
                                     └── 5.3 Review history
```

---

## Verification Commands

```bash
# Phase 1: Rust server tests
cargo test -p leindex-core --lib tracklens

# Phase 2: UI build
cd packages/tracklens-editor && npm run build

# Phase 3-5: TypeScript
cd pi-maestro && npm test

# Integration: End-to-end
maestro tracklens review --file maestro/tracks/test/spec.md --no-browser
```
