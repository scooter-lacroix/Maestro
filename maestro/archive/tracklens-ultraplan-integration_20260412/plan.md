# TrackLens × Ultraplan Integration — Implementation Plan

## Phase 1: Server Infrastructure (Foundation)

> **Blocks:** Phase 2, Phase 3, Phase 5
> **Files touched:** `src/leindex/src/tracklens/types.rs`, `src/leindex/src/tracklens/server.rs`, `src/leindex/src/tracklens/mod.rs`

- [x] Task 1.1: Add `TrackLensPhase` enum and `PhaseMetadata` to `types.rs`
  - Add `TrackLensPhase` enum: `Launching`, `Loading`, `Reviewing`, `Editing`, `Decided` with `serde(rename_all = "snake_case")`
  - Add `PhaseMetadata` struct: `review_duration_ms`, `edit_count`, `annotation_count`, `iteration`
  - Add `edited_content: Option<String>` and `phase_metadata: Option<PhaseMetadata>` to `TrackLensDecision` (both `skip_serializing_if`)
  - Update `test_decision_serialization` for new fields
  - Add `test_decision_with_edited_content` test
  - Write failing tests first (Red phase), then implement (Green phase)
  - Verify: `cargo test -p leindex-core --lib tracklens::types`

- [x] Task 1.2: Add phase tracking watch channel to `ServerState`
  - Add `phase_tx: watch::Sender<TrackLensPhase>`, `phase_rx: watch::Receiver<TrackLensPhase>` to `ServerState`
  - Add `iteration: Arc<AtomicU32>` to `ServerState`
  - Update imports: `use super::types::{ReviewMode, TrackLensDecision, TrackLensPhase}`
  - Initialize `(phase_tx, phase_rx) = watch::channel(TrackLensPhase::Launching)` in `TrackLensServer::new()`
  - Add fields to `ServerState` construction in `new()`
  - Add methods: `set_phase()`, `current_phase()`, `wait_for_phase_change()`
  - Write failing tests first, then implement
  - Verify: `cargo test -p leindex-core --lib tracklens::server`

- [x] Task 1.3: Add phase and content-update HTTP endpoints
  - Add routes: `GET /api/phase`, `POST /api/phase`, `POST /api/content`
  - Implement `get_phase` handler (returns current phase as JSON)
  - Implement `SetPhaseRequest` struct + `set_phase` handler
  - Implement `update_content` handler (replaces `ReviewContent` in `RwLock`)
  - Write failing tests first, then implement
  - Verify: `cargo test -p leindex-core --lib tracklens::server`

- [x] Task 1.4: Multi-round review (reset endpoint)
  - Add route: `POST /api/reset`
  - Implement `reset_review` handler: clears decision, increments iteration, resets phase to `Reviewing`
  - Add `reset_for_resubmit(new_content)` method on `TrackLensServer`
  - Add `iteration()` getter method
  - Write failing tests first, then implement
  - Verify: `cargo test -p leindex-core --lib tracklens::server`

- [x] Task 1.5: Graceful shutdown
  - Add `shutdown_tx: watch::Sender<bool>`, `shutdown_rx: watch::Receiver<bool>` to `ServerState`
  - Initialize in `new()`: `watch::channel(false)`
  - Replace blind `tokio::spawn` with `axum::serve().with_graceful_shutdown()` watching shutdown channel
  - Add `POST /api/shutdown` endpoint + `shutdown_server` handler
  - Add `TrackLensServer::shutdown()` method
  - Write failing tests first, then implement
  - Verify: `cargo test -p leindex-core --lib tracklens::server`

- [x] Task 1.6: Update `mod.rs` re-exports
  - Verify `pub use types::*` re-exports `TrackLensPhase`, `PhaseMetadata`
  - No changes needed if `pub use types::*` exists (verify and confirm)
  - Verify: `cargo test -p leindex-core --lib tracklens`

- [x] Task: Maestro - User Manual Verification 'Phase 1: Server Infrastructure' (Protocol in workflow.md)

---

## Phase 2: UI Enhancements

> **Blocks:** Phase 3 (workflow integration needs editing support)
> **Files touched:** `packages/tracklens-editor/src/App.tsx`, `packages/tracklens-editor/src/main.tsx`, build config

- [x] Task 2.0: Fix annotation serialization bug (BUG-1)
  - Root cause: `body.annotations = JSON.stringify(annotations)` double-encodes → server gets string not array → 422
  - Fix `handleApprove` (line 438): remove `JSON.stringify`, send array directly
  - Fix `handleDeny` (line 476): same fix
  - Fix `handleFeedback` (line 510): same fix
  - Update TypeScript type: `annotations?: string` → `annotations?: Annotation[]`
  - Also remove or update the misleading "Annotations Won't Be Sent" warning for claude-code origin since annotations WILL now be sent correctly
  - Verify: Approve with annotations succeeds (no 422)

- [x] Task 2.1: Add inline editing mode with CodeMirror
  - Add CodeMirror as dependency: `cd packages/tracklens-editor && bun add @codemirror/view @codemirror/state @codemirror/lang-markdown`
  - Add state: `editMode`, `editedMarkdown`
  - Add Edit/Preview toggle button in header
  - Conditionally render: CodeMirror editor (edit mode) vs AnnotationView (review mode)
  - Include `edited_content` in Approve decision POST
  - POST phase change to server on mode toggle: `editing` / `reviewing`
  - Write failing tests for state transitions, then implement
  - Verify: `cd packages/tracklens-editor && bun run dev`

- [x] Task 2.2: Add phase indicator in UI header
  - Add `phase` state, poll `GET /api/phase` every 2 seconds
  - Display phase badge: "Reviewing", "Editing", "Decided"
  - Style with appropriate colors per phase
  - Verify: Visual confirmation in browser

- [x] Task 2.3: Add keyboard shortcuts
  - `Ctrl/Cmd+Enter` → Approve
  - `Ctrl/Cmd+Shift+Enter` → Deny
  - `Ctrl/Cmd+E` → Toggle edit mode
  - Add shortcut hints in footer
  - Write event handler tests, then implement
  - Verify: Keyboard input triggers correct actions

- [x] Task 2.4: Build and bundle
  - Build React app: `cd packages/tracklens-editor && bun run build`
  - Copy output to `crates/cli/dist/tracklens-editor.html`
  - Verify `find_bundle_dir()` resolves the new bundle
  - Run server integration tests: `cargo test -p leindex-core --lib tracklens`
  - Verify: `maestro track-lens review --file <test-file> --no-browser` loads bundle

- [ ] Task: Maestro - User Manual Verification 'Phase 2: UI Enhancements' (Protocol in workflow.md)

---

## Phase 3: Workflow Integration

> **Blocks:** Phase 4 (keyword detection needs workflows to exist)
> **Files touched:** `amp-cli/skills/maestro/SKILL.md`, `claude-code/skills/maestro/SKILL.md`, `gemini-cli/skills/maestro/SKILL.md`, pi-maestro command files, `pi-maestro/src/tracklens/extension/tools.ts`

- [x] Task 3.0: Add TrackLens Review Protocol to maestro skill (LLM-facing)
  - **STATUS: DONE** — TrackLens Review Protocol section added to all three SKILL.md copies
  - Covers: when to call `tracklens_review`, `tracklens_walkthrough`, denial handling, approval handling
  - No further work needed

- [x] Task 3.1: Integrate TrackLens into `maestro:newTrack`
  - **STATUS: DONE** — TrackLens review checkpoints already present in newTrack workflow
  - Spec review at step 3.7, plan review at step 4.6, consolidated review at step 5.4
  - Handles approval/denial with annotations and manual fallback

- [x] Task 3.2: Integrate TrackLens into `maestro:setup`
  - Added TrackLens review step after setup doc generation in `pi-maestro/src/commands/setup.ts`
  - Combines product.md, tech-stack.md, workflow.md into single review document
  - Handles edited_content: parses sections back into individual files
  - Optional: silently skips if TrackLens server unavailable

- [x] Task 3.3: Strengthen `maestro:implement` integration
  - Made walkthrough review mandatory — removed `isTrackLensEnabled()` guard
  - Walkthrough section now always included as step 4.0, finalize as step 5.0
  - TrackLens integration section simplified (no conditional toggle)

- [x] Task 3.4: Integrate TrackLens into `maestro:orchestrate`
  - Removed `isTrackLensEnabled()` import and guard
  - Sub-track walkthrough notification now unconditional
  - Master track walkthrough via `spawnSync("maestro", ["tracklens", "walkthrough", ...])` now unconditional
  - Walkthrough blocks orchestration until approved (returns early if status !== 0)
  - Verify: `/maestro:orchestrate` pauses for walkthrough review

- [x] Task 3.5: Add `tracklens_code_review` tool
  - Registered in `pi-maestro/src/tracklens/extension/tools.ts`
  - Parameters: `gitRef` (string, default "HEAD"), `files` (array, optional)
  - Generates diff via `execSync('git diff HEAD')`, launches TrackLens in code-review mode
  - Returns structured annotations on deny
  - Three-tier fallback (server unavailable → HTML not built → error)

- [x] Task 3.6: Add `seedContent` parameter to `tracklens_review`
  - Added `seedContent` optional parameter to tool schema
  - When provided, prefixes content with `<!-- tracklens:editable -->` marker
  - Error message updated to include `seedContent` as valid input option

- [ ] Task: Maestro - User Manual Verification 'Phase 3: Workflow Integration' (Protocol in workflow.md)

---

## Phase 4: Keyword Detection & Auto-Trigger

> **Depends on:** Phase 3 (workflow integration must exist)
> **Files touched:** New file `pi-maestro/src/tracklens/keyword.ts`, `pi-maestro/src/index.ts`

- [x] Task 4.1: Port keyword detection from Ultraplan
  - Create `pi-maestro/src/tracklens/keyword.ts`
  - Implement `findKeywordTriggerPositions()` with delimiter-aware, path-aware, question-aware filtering
  - Keywords: "tracklens", "review this"
  - Export `findTrackLensTriggerPositions()`, `hasTrackLensKeyword()`, `hasReviewTrigger()`, `replaceTrackLensKeyword()`
  - Skip false positives: quoted ranges, code spans, path-like contexts, question suffixes, slash commands
  - Write failing tests for all edge cases, then implement
  - Verify: `cd pi-maestro && bun test tracklens/keyword`

- [x] Task 4.2: Wire keyword detection into message processing
  - Import keyword detection in `pi-maestro/src/index.ts` or global message hook
  - Check user messages for TrackLens keywords before sending to agent
  - If keyword found AND recent document exists (within 10 min) → auto-invoke appropriate TrackLens tool
  - If keyword found AND no recent document → let message through to model
  - Add `tracklensAutoTriggered` metadata flag to prevent double-trigger
  - Write failing tests, then implement
  - Verify: User message "tracklens" auto-triggers review when recent doc exists

- [x] Task 4.3: Seed plan support in tool invocation
  - Server side: detect `<!-- tracklens:editable -->` marker, set phase to `Editing`
  - UI side: detect marker on mount, auto-enter edit mode, strip marker from display
  - End-to-end: seed content → editable editor → approve with edited_content
  - Write failing tests, then implement
  - Verify: Seed content opens in editable mode

- [ ] Task: Maestro - User Manual Verification 'Phase 4: Keyword Detection & Auto-Trigger' (Protocol in workflow.md)

---

## Phase 5: Agent-Side Feedback Loop

> **Depends on:** Phase 1, Phase 3
> **Files touched:** `pi-maestro/src/tracklens/extension/tools.ts`, new files

- [x] Task 5.1: Structured feedback formatting on denial
  - Implement `formatDenialForAgent()` in `tools.ts`
  - Format: header → general feedback → annotations grouped by severity (ERROR > WARNING > INFO)
  - Each annotation: severity badge, line number, quoted selection text, comment
  - Footer with action instruction
  - Return structured feedback as tool result with `approved: false`
  - Write failing tests, then implement
  - Verify: Denied review returns well-formatted markdown feedback

- [x] Task 5.2: Phase reporting to agent
  - Create `pi-maestro/src/tracklens/phaseReporter.ts`
  - Implement `startPhaseReporter()` with configurable poll interval (default 3s)
  - Uses `AbortSignal` for cleanup
  - `onPhaseChange` callback reports to agent context
  - Wire into `tracklens_review` tool: start reporter after server launch, abort after decision
  - Write failing tests, then implement
  - Verify: Agent sees "User is reviewing" / "User is editing" status updates

- [x] Task 5.3: Review history persistence
  - Create `pi-maestro/src/tracklens/history.ts`
  - `ReviewHistoryEntry` interface: timestamp, documentType, approved, annotationCount, feedback, editedContent, reviewDurationMs, iteration
  - `loadReviewHistory()` and `appendReviewEntry()` functions
  - Persist to `maestro/tracks/<id>/review-history.json`
  - `formatHistoryForAgent()` — last 5 entries formatted as context
  - Wire into `tracklens_review` tool: auto-persist after each decision
  - Write failing tests, then implement
  - Verify: Review history file created and updated after each review

- [ ] Task: Maestro - User Manual Verification 'Phase 5: Agent-Side Feedback Loop' (Protocol in workflow.md)

---

## Dependency Graph

```
Phase 1 (Server)
  1.1 Types ──────────┐
  1.2 Phase channel ──┤
  1.3 HTTP endpoints ─┤── Phase 2 (UI)
  1.4 Reset endpoint ─┤     2.0 BUG FIX (annotation serialization)
  1.5 Shutdown ────────┤     2.1 Edit mode
  1.6 Re-exports ─────┘     2.2 Phase indicator
                             2.3 Keyboard shortcuts
                             2.4 Build bundle
                                   │
                             Phase 3 (Workflows)
                               3.0 Skill ✅ DONE
                               3.1 newTrack
                               3.2 setup
                               3.3 implement
                               3.4 orchestrate
                               3.5 code-review tool
                               3.6 seedContent param
                                   │
                             Phase 4 (Keywords)
                               4.1 Detection module
                               4.2 Message hook
                               4.3 Seed plan flow
                                   │
                             Phase 5 (Feedback)
                               5.1 Structured denial
                               5.2 Phase reporter
                               5.3 Review history
```

## Verification Commands

```bash
# Phase 1: Rust server tests
cargo test -p leindex-core --lib tracklens

# Phase 2: UI build
cd packages/tracklens-editor && bun run build

# Phase 3-5: TypeScript
cd pi-maestro && bun test

# Integration: End-to-end
maestro track-lens review --file maestro/tracks/test/spec.md
```
