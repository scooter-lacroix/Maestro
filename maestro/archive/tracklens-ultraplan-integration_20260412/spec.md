# TrackLens × Ultraplan Integration — Specification

## Overview

Integrate Ultraplan's browser-based review capabilities into Maestro's TrackLens system, making TrackLens the universal review checkpoint at every phase of the Maestro workflow. This covers server infrastructure enhancements, UI inline editing, workflow integration across all Maestro commands, keyword-based auto-triggering, and an agent-side feedback loop.

## Track Type

Feature — Enhancement to existing TrackLens system

## Background

TrackLens is Maestro's local-first review system with an Axum HTTP server, React SPA, and rich annotation model. It currently only fires at walkthrough completion. Ultraplan (Claude Code's remote plan drafting feature) provides keyword auto-triggering, real-time phase tracking, multi-turn browser editing, and bidirectional feedback. This track adapts those patterns for local-first TrackLens.

## Functional Requirements

### FR-1: Server Infrastructure (Phase 1)

**FR-1.1: Phase Tracking**
- Add `TrackLensPhase` enum with states: `Launching`, `Loading`, `Reviewing`, `Editing`, `Decided`
- Add `watch::Sender<TrackLensPhase>` / `watch::Receiver<TrackLensPhase>` to `ServerState`
- Expose `GET /api/phase` and `POST /api/phase` endpoints
- Add `TrackLensServer::set_phase()`, `current_phase()`, `wait_for_phase_change()` methods

**FR-1.2: Content Update Endpoint**
- Add `POST /api/content` accepting `ReviewContent` to update in-flight review
- Enables seed plan refinement — agent sends draft, user edits, agent sends updated draft
- Content must be replaceable without restarting the server

**FR-1.3: Enhanced Decision Model**
- Add `edited_content: Option<String>` to `TrackLensDecision` — user's inline-edited content
- Add `phase_metadata: Option<PhaseMetadata>` with `review_duration_ms`, `edit_count`, `annotation_count`, `iteration`
- Backward compatible — both fields are `Option<T>` with `skip_serializing_if`

**FR-1.4: Multi-Round Review**
- Add `POST /api/reset` endpoint to clear decision state without restarting server
- Track review iteration count via `AtomicU32` in `ServerState`
- `reset_for_resubmit()` method for agent to update content + clear decision + increment iteration

**FR-1.5: Graceful Shutdown**
- Add `shutdown_tx/shutdown_rx` watch channel to `ServerState`
- Replace blind `tokio::spawn` with `axum::serve().with_graceful_shutdown()`
- Add `POST /api/shutdown` endpoint and `TrackLensServer::shutdown()` method

**FR-1.6: Module Re-exports**
- Update `mod.rs` to re-export `TrackLensPhase`, `PhaseMetadata` via `pub use types::*`

### FR-2: UI Enhancements (Phase 2)

**FR-2.1: Inline Editing Mode**
- Add CodeMirror editor component for markdown editing with syntax highlighting
- Toggle button in header switches between annotation view (read-only) and edit view (CodeMirror)
- Edit state tracked in React, included in decision POST as `edited_content`
- Phase changes posted to server when toggling modes

**FR-2.2: Phase Indicator**
- Poll `GET /api/phase` every 2 seconds
- Display phase badge in header: "Reviewing", "Editing", "Decided"
- Visual feedback that server is tracking user activity

**FR-2.3: Keyboard Shortcuts**
- `Ctrl/Cmd+Enter` → Approve
- `Ctrl/Cmd+Shift+Enter` → Deny
- `Ctrl/Cmd+E` → Toggle edit mode
- Shortcut hints displayed in footer

**FR-2.4: Build and Bundle**
- Build React app → output to `crates/cli/dist/tracklens-editor.html`
- Verify `find_bundle_dir()` resolves the new bundle
- Server integration tests must pass with new bundle

**FR-2.5: Bug Fix — Annotation Serialization**
- Fix double-stringified `annotations` field in approve/deny/feedback handlers
- Root cause: `body.annotations = JSON.stringify(annotations)` → 422 from server
- Fix: Remove `JSON.stringify`, send as array, update TypeScript type
- Affects `App.tsx` lines 438, 476, 510

### FR-3: Workflow Integration (Phase 3)

**FR-3.0: Skill Protocol (DONE)**
- TrackLens Review Protocol added to all three SKILL.md copies
- Covers: when to call `tracklens_review`, `tracklens_walkthrough`, denial handling, approval handling

**FR-3.1: maestro:newTrack Integration**
- After spec.md draft → call `tracklens_review` with `documentType: "spec.md"`
- After plan.md draft → call `tracklens_review` with `documentType: "plan.md"`
- On deny → incorporate annotations/feedback, re-draft, re-review (max 3 iterations)
- On approve with `edited_content` → use edited version as final

**FR-3.2: maestro:setup Integration**
- After generating setup docs → call `tracklens_review` with combined markdown
- Optional: review each doc separately

**FR-3.3: maestro:implement Integration**
- Make walkthrough review mandatory (remove `isTrackLensEnabled()` guard)
- Use remediation loop from `remediation.ts`
- Report phase state back to agent during review

**FR-3.4: maestro:orchestrate Integration**
- After each sub-track completion → trigger walkthrough review
- After master track completion → trigger aggregate walkthrough review
- Block orchestration until review is approved

**FR-3.5: Code Review Tool**
- Add `tracklens_code_review` tool to pi-maestro
- Accepts `gitRef` and optional `files` array
- Generates diff, opens TrackLens in code-review mode
- Returns annotations as structured feedback

**FR-3.6: Seed Content Parameter**
- Add `seedContent` parameter to `tracklens_review` tool
- Pre-populates editor with existing draft for refinement
- Ultraplan's `seedPlan` pattern adapted for local review

### FR-4: Keyword Detection & Auto-Trigger (Phase 4)

**FR-4.1: Keyword Detection Module**
- New file `pi-maestro/src/tracklens/keyword.ts`
- Port Ultraplan's `findKeywordTriggerPositions()` with TrackLens keywords
- Keywords: "tracklens", "review this", "show review"
- Filter: quoted ranges, code spans, path-like contexts, question suffixes, slash commands

**FR-4.2: Message Processing Hook**
- Check user messages for TrackLens keywords before sending to agent
- If keyword found and recent document exists → auto-invoke TrackLens
- If keyword found and no recent document → show usage instructions

**FR-4.3: Seed Plan Support**
- `tracklens_review` tool accepts optional `seedContent` parameter
- Seed content shown in editor as initial editable draft
- User can modify and approve edited version

### FR-5: Agent-Side Feedback Loop (Phase 5)

**FR-5.1: Structured Feedback Injection**
- On deny → format annotations as structured XML/markdown for agent consumption
- Include positions, severity, text selections
- Agent receives feedback as tool result with `approved: false` and remediation list

**FR-5.2: Phase Reporting**
- New file `pi-maestro/src/tracklens/phaseReporter.ts`
- Poll server phase state, report to agent context
- Agent sees real-time status: "User is reviewing spec.md (3 annotations)"

**FR-5.3: Review History**
- New file `pi-maestro/src/tracklens/history.ts`
- Store review history per track/document in `maestro/tracks/<id>/review-history.json`
- Agent can reference prior feedback when re-drafting

## Non-Functional Requirements

**NFR-1: Security**
- CORS locked to localhost with dynamic port (existing — maintain)
- Path traversal validation on track IDs (existing — maintain)
- Request body size limits (existing 100KB — maintain)
- No new remote dependencies — everything stays local-first

**NFR-2: Performance**
- Phase polling interval: 2 seconds (UI), configurable
- CodeMirror lazy-loaded to avoid impacting initial page load
- Review history stored as JSON, not in database

**NFR-3: Compatibility**
- No breaking changes to `TrackLensDecision` — new fields are `Option<T>`
- Existing CLI commands (`tracklens review`, `tracklens walkthrough`, `tracklens code-review`) unchanged
- Pi-Maestro tools backward-compatible — new parameters optional
- Existing annotation flow unchanged

**NFR-4: Testability**
- Unit tests for all new Rust types (serialization round-trips)
- Integration tests for new HTTP endpoints
- UI tests for keyboard shortcuts and edit mode
- TDD workflow per project standards (>98% coverage target)

## Acceptance Criteria

1. Every Maestro workflow phase that generates a reviewable document triggers TrackLens automatically
2. User can edit content inline in the browser using CodeMirror (not just annotate)
3. Agent receives structured feedback from denials and automatically iterates
4. Review phase is visible to the agent in real-time via phase tracking
5. Keyword detection works for "tracklens" and "review this" without false positives in quoted/code/path contexts
6. Full review history is persisted per track in `review-history.json`
7. No regression in existing TrackLens CLI functionality
8. All new code has >98% test coverage
9. Server shuts down gracefully (no orphaned processes)
10. Multi-round review works without restarting server
11. **Approve/Deny works with annotations present** (BUG-1 fix)

## Out of Scope

- Remote CCR session support (TrackLens stays local-first)
- 30-minute timeouts (local review uses shorter configurable defaults)
- Remote execution target (plans always return to local agent)
- Obsidian vault integration changes (existing functionality preserved)
- Mobile/responsive UI (desktop browser only)
- Internationalization (English-only)
