# Subtrack 04: Parallel Execution UI - Specification

**Track ID:** 04-conductor-phase4-parallel-ui
**Parent:** conductor-ralph-parity_20260213
**Phase:** 4
**Status:** New
**Dependencies:** 01-conductor-phase1-ui-primitives

---

## Objective

Add UI components for parallel execution visualization including parallel state types, worker view, merge queue, and conflict resolution panel.

---

## Requirements

### R1: Parallel State Types
- **R1.1:** Add `ParallelStatus` enum (Idle, Running, Paused, Merging, Complete)
- **R1.2:** Add `WorkerStatus` enum (Idle, Working, Waiting, Complete, Error)
- **R1.3:** Add `MergeStatus` enum (Waiting, Merging, Conflicted, Complete)
- **R1.4:** Add to `orchestrate/model.rs`

### R2: Parallel Worker State
- **R2.1:** Add `ParallelWorkerState` struct (id, task_id, status, progress, output)
- **R2.2:** Add `MergeQueueEntry` struct (worker_id, task_id, status, conflicts)
- **R2.3:** Add `ConflictInfo` struct (file, ours, theirs, resolution_method)
- **R2.4:** Add `ParallelGroupInfo` for parallel task grouping

### R3: Parallel View Component
- **R3.1:** Create `parallel_view.rs` component
- **R3.2:** Show worker list with status icons
- **R3.3:** Show merge queue with progress
- **R3.4:** Show overall parallel progress
- **R3.5:** Enable Alt+4 to switch to parallel view

### R4: Conflict Resolution Panel
- **R4.1:** Create `conflict_panel.rs` modal
- **R4.2:** Show conflicting file content
- **R4.3:** Show ours vs theirs diff
- **R4.4:** Provide resolution options (Accept Ours, Accept Theirs, AI Resolve)

### R5: Parallel ConductorEvents
- **R5.1:** Add `ConductorEvent::ParallelStarted`
- **R5.2:** Add `ConductorEvent::WorkerStatusChanged`
- **R5.3:** Add `ConductorEvent::MergeQueueUpdated`
- **R5.4:** Add `ConductorEvent::ConflictDetected`
- **R5.5:** Add `ConductorEvent::ConflictResolved`
- **R5.6:** Add `ConductorEvent::ParallelCompleted`
- **R5.7:** Add `ConductorEvent::ParallelPaused`
- **R5.8:** Add `ConductorEvent::ParallelResumed`
- **R5.9:** Add `ConductorEvent::WorkerOutput`
- **R5.10:** Add `ConductorEvent::WorkerError`
- **R5.11:** Add `ConductorEvent::MergeProgress`
- **R5.12:** Add `ConductorEvent::MergeConflict`

### R6: Parallel Keybindings
- **R6.1:** `Ctrl+P` → Pause/Resume parallel execution
- **R6.2:** `R` → Accept ours (in conflict panel)
- **R6.3:** `S` → Accept theirs (in conflict panel)
- **R6.4:** `O` → AI resolve (in conflict panel)
- **R6.5:** `T` → Skip file (in conflict panel)
- **R6.6:** `4` → Switch to parallel view (Alt+4)

### R7: Header/Footer/Dashboard Updates
- **R7.1:** Show parallel status in header (when active)
- **R7.2:** Show worker count in dashboard
- **R7.3:** Show merge queue length in dashboard
- **R7.4:** Update footer key hints for parallel mode

---

## Acceptance Criteria

- [ ] Parallel state types compile and serialize correctly
- [ ] Parallel view shows workers and merge queue
- [ ] Conflict panel renders conflicts with resolution options
- [ ] All 12 parallel events emit correctly
- [ ] Keybindings functional for parallel control
- [ ] Header/footer update for parallel mode

---

## Dependencies

- 01-conductor-phase1-ui-primitives (modals)

## Blocks

- 05-conductor-phase5-parallel-polling
