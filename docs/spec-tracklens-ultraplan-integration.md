# TrackLens × Ultraplan Integration — Spec Bible & Implementation Plan

> **Goal:** Adopt the best aspects of Claude Code's Ultraplan feature into Maestro's TrackLens system, making TrackLens the universal review checkpoint at every phase of the Maestro workflow.

---

## 1. Executive Summary

Ultraplan provides a polished browser-based plan review experience with keyword auto-triggering, real-time phase tracking, multi-turn editing in the browser, and bidirectional feedback flow. TrackLens already has the server infrastructure (local Axum), rich annotation model, and review UI — but it's only wired in at walkthrough completion. This spec bridges the gap.

### What We're Taking from Ultraplan

| Ultraplan Feature | TrackLens Adaptation |
|---|---|
| Keyword auto-trigger (`hasUltraplanKeyword`) | Keyword detection for `tracklens` / `review` in user messages |
| Seed plan injection (refine existing plan) | Pre-populate editor with existing spec/plan for iterative refinement |
| Phase tracking (running → needs_input → plan_ready) | Phase state reported to agent: `generating → reviewing → approved/denied` |
| UltraplanChoiceDialog (execute/dismiss) | Inline agent-facing result with approved plan or denial annotations |
| UltraplanLaunchDialog (pre-launch confirmation) | Optional confirm dialog before opening browser (configurable) |
| `buildUltraplanPrompt()` seed+instructions assembly | `buildReviewPayload()` that assembles content + metadata + prior feedback |
| Detached poll with `onPhaseChange` callbacks | `TrackLensServer::poll_phase()` emitting phase updates to agent |
| Multi-turn in browser (reject → iterate → re-approve) | Support deny→edit→resubmit loop in TrackLens UI without restarting server |

### What We're NOT Taking

- **Remote CCR sessions**: TrackLens stays local-first (no Anthropic auth dependency)
- **30-minute timeouts**: Local review has configurable but shorter defaults (5 min, extendable)
- **Remote execution target**: No "execute in CCR" path — plans always return to local agent

---

## 2. Integration Points — Where TrackLens Gets Invoked

### 2.1 `maestro:setup` (Project Initialization)

**When:** After generating `product.md`, `tech-stack.md`, `workflow.md`
**What to review:** Each generated document individually or as a combined setup review
**Mode:** `ReviewMode::Review`
**Behavior:**
- Agent generates setup docs → calls `tracklens_review` with the combined markdown
- User reviews/annotates in browser → approve or deny with feedback
- On deny: agent incorporates feedback and regenerates

### 2.2 `maestro:newTrack` (Track Creation)

**When:** After spec.md is drafted, and again after plan.md is drafted
**What to review:** `spec.md` first, then `plan.md` (two sequential reviews)
**Mode:** `ReviewMode::Review`
**Behavior:**
- Agent drafts spec → `tracklens_review(markdown, documentType: "spec.md")`
- User approves spec → agent drafts plan → `tracklens_review(markdown, documentType: "plan.md")`
- On deny of either: agent incorporates annotations and re-drafts
- **Seed plan support:** If user has a rough plan, pass it as initial content for the editor (à la Ultraplan's `seedPlan`)

### 2.3 `maestro:implement` (Track Implementation)

**When:** After all tasks are completed, before marking track as done
**What to review:** Auto-generated walkthrough
**Mode:** `ReviewMode::Review` (walkthrough mode)
**Behavior:**
- Agent completes implementation → calls `tracklens_walkthrough(trackId)`
- Walkthrough is generated with completed tasks, changed files, diffs
- User reviews → approve completes track, deny triggers remediation loop
- **This already partially exists** — but needs to be made mandatory and robust

### 2.4 `maestro:orchestrate` (Master Track Orchestration)

**When:** After each sub-track completes, and at master track completion
**What to review:** Sub-track walkthrough + master track summary
**Mode:** `ReviewMode::Review`
**Behavior:**
- Each sub-track completion triggers walkthrough review
- Master track completion triggers aggregate walkthrough
- Orchestrator pauses until review is approved

### 2.5 Code Changes Review (Any Phase)

**When:** After significant code changes in any phase
**What to review:** Git diff
**Mode:** `ReviewMode::CodeReview`
**Behavior:**
- Agent generates or applies code changes → `tracklens_review(diff, mode: "code-review")`
- User reviews diff with annotation support
- On deny: agent addresses specific annotated issues

### 2.6 Auto-Trigger via Keyword Detection

**When:** User types "tracklens" or "review this" in their message
**What to review:** Context-dependent (last generated document)
**Behavior:**
- Keyword detection in user input (adapted from Ultraplan's `findKeywordTriggerPositions`)
- If agent has recently generated a document, auto-invoke TrackLens review
- If no recent document, show usage instructions

---

## 3. Architecture — Enhanced TrackLens System

### 3.1 Server Enhancements (Rust: `server.rs`)

```
New capabilities:
├── Phase tracking (watch channel: phase_tx/phase_rx)
├── Content update endpoint (POST /api/content) — for seed plan edits
├── In-browser edit support (POST /api/save-edits) — save inline edits
├── Multi-round review (reset decision state without restarting server)
└── Graceful shutdown endpoint (POST /api/shutdown)
```

### 3.2 New Phase State Machine

```
TrackLensPhase:
  Launching    → Server starting, browser opening
  Loading      → Client connected, content loading
  Reviewing    → User is reviewing/annotating
  Editing      → User is editing content inline
  Decided      → User submitted decision
```

### 3.3 Enhanced Decision Model

```rust
pub struct TrackLensDecision {
    pub behavior: DecisionBehavior,    // Allow | Deny
    pub annotations: Option<Vec<Annotation>>,
    pub feedback: Option<String>,
    pub autonomy_mode: Option<AutonomyMode>,
    pub edited_content: Option<String>,  // NEW: inline-edited content
    pub phase_metadata: Option<PhaseMetadata>,  // NEW: timing info
}

pub struct PhaseMetadata {
    pub review_duration_ms: u64,
    pub edit_count: u32,
    pub annotation_count: u32,
}
```

### 3.4 UI Enhancements (React: `tracklens-editor`)

- **Inline editing mode**: Toggle between read-only annotation mode and full markdown editing
- **Split view**: Side-by-side original vs. edited content
- **Auto-save draft**: Persist edits to localStorage, recover on page refresh
- **Phase indicator**: Show current phase in header bar
- **Keyboard shortcuts**: `Ctrl+Enter` approve, `Ctrl+Shift+Enter` deny, `Ctrl+E` toggle edit

---

## 4. Implementation Plan — Blocking Tasks

### Phase 1: Server Infrastructure (Foundation)

> **Blocks:** Everything else. Must be completed first.

#### Task 1.1: Add Phase Tracking to TrackLensServer
- **File:** `src/leindex/src/tracklens/server.rs`
- **Work:** Add `phase_tx: watch::Sender<TrackLensPhase>`, `phase_rx: watch::Receiver<TrackLensPhase>`
- **Add endpoint:** `GET /api/phase` — returns current phase
- **Add endpoint:** `POST /api/phase` — update phase from client (e.g., "editing")
- **Add method:** `TrackLensServer::poll_phase()` — async poll that returns on phase change
- **Tests:** Unit test phase transitions, integration test with HTTP client

#### Task 1.2: Add Content Update Endpoint
- **File:** `src/leindex/src/tracklens/server.rs`
- **Work:** `POST /api/content` — accepts new `ReviewContent` to update in-flight review
- **Use case:** Seed plan refinement — agent sends draft, user edits, agent sends updated draft
- **Tests:** Content update replaces previous content, UI refreshes

#### Task 1.3: Add Edited Content to Decision
- **File:** `src/leindex/src/tracklens/types.rs`
- **Work:** Add `edited_content: Option<String>` to `TrackLensDecision`
- **Work:** Add `phase_metadata: Option<PhaseMetadata>` with timing info
- **Tests:** Serialization round-trip tests

#### Task 1.4: Multi-Round Review Support
- **File:** `src/leindex/src/tracklens/server.rs`
- **Work:** `POST /api/reset` — clears decision state, allows re-review without restarting server
- **Work:** Track review iteration count in server state
- **Tests:** Reset clears decision, wait_for_decision works again after reset

#### Task 1.5: Graceful Shutdown
- **File:** `src/leindex/src/tracklens/server.rs`
- **Work:** `POST /api/shutdown` — triggers server shutdown via tokio shutdown signal
- **Work:** `TrackLensServer::shutdown()` method for programmatic shutdown
- **Tests:** Server stops accepting connections after shutdown

---

### Phase 2: UI Enhancements (Blocks Phase 3)

> **Blocks:** Integration with workflows (needs editing support first)

#### Task 2.1: Inline Editing Mode
- **File:** `packages/tracklens-editor/src/App.tsx`
- **Work:** Add edit toggle button in header
- **Work:** Switch between `AnnotationView` (read-only + annotations) and `EditView` (CodeMirror/textarea)
- **Work:** Track edits in React state, include in decision POST as `edited_content`
- **Tests:** Visual verification, edit state persistence

#### Task 2.2: Phase Indicator in UI
- **File:** `packages/tracklens-editor/src/App.tsx`
- **Work:** Poll `GET /api/phase` or use SSE for real-time phase display
- **Work:** Show phase badge in header: "Reviewing", "Editing", "Submitting"
- **Work:** POST phase changes to server when user switches modes

#### Task 2.3: Keyboard Shortcuts
- **File:** `packages/tracklens-editor/src/App.tsx`
- **Work:** `Ctrl+Enter` → Approve, `Ctrl+Shift+Enter` → Deny, `Ctrl+E` → Toggle edit
- **Work:** Show shortcuts in footer/tooltip
- **Tests:** Key event handlers fire correct actions

#### Task 2.4: Build and Bundle
- **Work:** Build React app → output to `crates/cli/dist/tracklens-editor.html` (single-file SPA)
- **Work:** Update `find_bundle_dir()` to also check `~/.maestro/tracklens/` for installed bundles
- **Tests:** Bundle loads in browser, all endpoints work

---

### Phase 3: Workflow Integration (Blocks Phase 4)

> **Blocks:** Keyword detection and auto-trigger

#### Task 3.1: Integrate into `maestro:newTrack`
- **File:** `pi-maestro/src/commands/newTrack.ts`
- **Work:** After spec draft, call `tracklens_review` tool with `documentType: "spec.md"`
- **Work:** After plan draft, call `tracklens_review` tool with `documentType: "plan.md"`
- **Work:** On deny, inject feedback into agent context and re-draft
- **Work:** On approve with `edited_content`, use edited version as final

#### Task 3.2: Integrate into `maestro:setup`
- **File:** `pi-maestro/src/commands/setup.ts`
- **Work:** After generating setup docs, call `tracklens_review` with combined markdown
- **Work:** Optional: review each doc separately (product.md, tech-stack.md, workflow.md)

#### Task 3.3: Strengthen `maestro:implement` Integration
- **File:** `pi-maestro/src/commands/implement.ts`
- **Work:** Make walkthrough review mandatory (not just when `isTrackLensEnabled()`)
- **Work:** Use remediation loop from `pi-maestro/src/tracklens/walkthrough/remediation.ts`
- **Work:** Report phase state back to agent during review

#### Task 3.4: Integrate into `maestro:orchestrate`
- **File:** `pi-maestro/src/commands/orchestrate.ts`
- **Work:** After each sub-track completion, trigger walkthrough review
- **Work:** After master track completion, trigger aggregate walkthrough review
- **Work:** Block orchestration until review is approved

#### Task 3.5: Code Review Integration
- **File:** `pi-maestro/src/tracklens/extension/tools.ts`
- **Work:** Add `tracklens_code_review` tool that accepts a git ref
- **Work:** Generates diff, opens TrackLens in code-review mode
- **Work:** Returns annotations as structured feedback

---

### Phase 4: Keyword Detection & Auto-Trigger

> **Depends on:** Phase 3 (workflow integration must exist to auto-trigger into)

#### Task 4.1: Port Keyword Detection from Ultraplan
- **New file:** `pi-maestro/src/tracklens/keyword.ts`
- **Work:** Port `findKeywordTriggerPositions()` from Ultraplan's `keyword.ts`
- **Work:** Keywords: "tracklens", "review this", "show review"
- **Work:** Same delimiter-aware, path-aware, question-aware filtering
- **Tests:** Unit tests for all edge cases (quoted, path-like, question)

#### Task 4.2: Wire Keyword Detection into Message Processing
- **File:** `pi-maestro/src/index.ts` or message processing hook
- **Work:** Check user messages for TrackLens keywords before sending to agent
- **Work:** If keyword found and recent document exists, auto-invoke TrackLens
- **Work:** If keyword found and no recent document, show usage instructions

#### Task 4.3: Seed Plan Support
- **File:** `pi-maestro/src/tracklens/extension/tools.ts`
- **Work:** `tracklens_review` tool accepts optional `seedContent` parameter
- **Work:** Seed content is shown in the editor as initial editable draft
- **Work:** User can modify seed content and approve the edited version

---

### Phase 5: Agent-Side Feedback Loop (Polish)

> **Depends on:** Phase 3 and 4

#### Task 5.1: Structured Feedback Injection
- **File:** `pi-maestro/src/tracklens/extension/tools.ts`
- **Work:** On deny, format annotations as structured XML/markdown for agent consumption
- **Work:** Include annotation positions, severity, and text selections
- **Work:** Agent receives feedback as tool result with `approved: false` and structured remediation list

#### Task 5.2: Phase Reporting to Agent
- **New file:** `pi-maestro/src/tracklens/phaseReporter.ts`
- **Work:** Poll TrackLens server phase state and report to agent context
- **Work:** Agent sees: "User is reviewing spec.md in TrackLens (3 annotations so far)"
- **Work:** Similar to Ultraplan's `onPhaseChange` callback updating task state

#### Task 5.3: Review History
- **New file:** `pi-maestro/src/tracklens/history.ts`
- **Work:** Store review history (decision, annotations, edits) per track/document
- **Work:** Persist to `maestro/tracks/<id>/review-history.json`
- **Work:** Agent can reference prior review feedback when re-drafting

---

## 5. Data Flow — Complete Lifecycle

```
User says "create a new track for auth refactor"
  ↓
maestro:newTrack workflow starts
  ↓
Agent generates spec.md
  ↓
Agent calls tracklens_review(markdown, "spec.md")
  ↓
TrackLensServer::new() → server.start() → browser opens
  ↓
Phase: Launching → Loading → Reviewing
  ↓
User reads spec, adds 2 annotations, clicks Deny
  ↓
Decision: { behavior: Deny, annotations: [...], feedback: "..." }
  ↓
Server returns denial to tool → agent receives structured feedback
  ↓
Agent re-drafts spec incorporating feedback
  ↓
Agent calls tracklens_review(updated_markdown, "spec.md") — same server via /api/content + /api/reset
  ↓
User reviews updated spec, approves
  ↓
Decision: { behavior: Allow, edited_content: "..." }
  ↓
Agent saves edited version as final spec.md
  ↓
Agent generates plan.md → same cycle repeats for plan
  ↓
Track creation complete
```

---

## 6. Migration & Compatibility

- **TrackLens CLI commands** (`tracklens review`, `tracklens walkthrough`, `tracklens code-review`) continue to work unchanged
- **Pi-Maestro tools** (`tracklens_review`, `tracklens_walkthrough`) get new parameters but remain backward-compatible
- **Existing UI** gets editing mode added; annotation flow unchanged
- **No breaking changes** to `TrackLensDecision` — `edited_content` and `phase_metadata` are `Option<T>`

---

## 7. Success Criteria

1. Every Maestro workflow phase that generates a reviewable document triggers TrackLens
2. User can edit content inline in the browser (not just annotate)
3. Agent receives structured feedback from denials and automatically iterates
4. Review phase is visible to the agent in real-time (phase tracking)
5. Keyword detection works for "tracklens" and "review this" without false positives
6. Full review history is persisted per track
7. No regression in existing TrackLens CLI functionality
