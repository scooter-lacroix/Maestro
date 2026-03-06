# Subtrack 04: Parallel Execution UI - Implementation Plan

**Track:** 04-conductor-phase4-parallel-ui
**Parent:** conductor-ralph-parity_20260213
**Status:** New

---

## Phase 1: Parallel State Types (4 tasks)

### [x] Task 1.1: Add ParallelStatus enum
- Create `ParallelStatus` enum with Idle/Running/Paused/Merging/Complete variants
- Add Serialize/Deserialize
- Add to orchestrate/model.rs
- **Dependencies:** None
- **Deliverables:** ParallelStatus enum

### [x] Task 1.2: Add WorkerStatus enum
- Add serialization
- Add serialization
- **Dependencies:** Task 1.1
- **Deliverables:** WorkerStatus enum

### [x] Task 1.3: Add MergeStatus enum
- Create `MergeStatus` enum with Waiting/Merging/Conflicted/Complete variants
- Add serialization
- **Dependencies:** Task 1.2
- **Deliverables:** MergeStatus enum

### [x] Task 1.4: Add unit tests for state types
- Test serialization roundtrip for all enums
- Test default values
- **Dependencies:** Task 1.3
- **Deliverables:** Tests

---

## Phase 2: Parallel Worker State Structs (4 tasks)

### [x] Task 2.1: Add ParallelWorkerState struct
- id: String
- task_id: String
- status: WorkerStatus
- progress: f32 (0.0-1.0)
- output: Vec<String>
- started_at: Option<DateTime>
- **Dependencies:** Phase 1
- **Deliverables:** ParallelWorkerState

### [x] Task 2.2: Add MergeQueueEntry struct
- worker_id: String
- task_id: String
- status: MergeStatus
- conflicts: Vec<ConflictInfo>
- **Dependencies:** Task 2.1
- **Deliverables:** MergeQueueEntry

### [x] Task 2.3: Add ConflictInfo struct
- file: String
- ours: String (content preview)
- theirs: String (content preview)
- resolution_method: Option<String>
- **Dependencies:** Task 2.2
- **Deliverables:** ConflictInfo

### [x] Task 2.4: Add ParallelGroupInfo struct
- group_id: String
- task_ids: Vec<String>
- status: ParallelStatus
- workers: Vec<ParallelWorkerState>
- merge_queue: Vec<MergeQueueEntry>
- **Dependencies:** Task 2.3
- **Deliverables:** ParallelGroupInfo

---

## Phase 3: Parallel View Component (5 tasks)

### [x] Task 3.1: Create parallel_view.rs
- Define ParallelView struct
- Fields: group_info, selected_worker, scroll_offset
- Add to conductor/mod.rs
- **Dependencies:** Phase 2
- **Deliverables:** parallel_view.rs

### [x] Task 3.2: Implement worker list rendering
- Render list of workers with status icons
- Show task_id, progress bar, status
### [x] Task 3.2: Implement worker list rendering
- **Dependencies:** Task 3.1
- **Deliverables:** Worker list

### [x] Task 3.3: Implement merge queue rendering
- Show merge queue entries
- Display status icons
- Show conflict count if any
- **Dependencies:** Task 3.2
- **Deliverables:** Merge queue

### [x] Task 3.4: Implement progress overview
- Show overall parallel progress
- Workers complete / total workers
- Merge queue position
- **Dependencies:** Task 3.3
- **Deliverables:** Progress overview

### [ ] Task 3.5: Add Alt+4 view switch
- Add Parallel to DetailsViewMode enum
- Switch to parallel view on Alt+4
- **Dependencies:** Task 3.4
- **Deliverables:** View switch

---

## Phase 4: Conflict Resolution Panel (5 tasks)

### [x] Task 4.1: Create conflict_panel.rs
- Define ConflictPanel struct
- Fields: conflict, visible, selected_option
- Add to conductor/mod.rs
- **Dependencies:** Phase 2
- **Deliverables:** conflict_panel.rs

### [x] Task 4.2: Implement conflict display
- Show conflicting file path
- Show ours content preview
- Show theirs content preview
- Highlight differences
- **Dependencies:** Task 4.1
- **Deliverables:** Conflict display

### [x] Task 4.3: Implement resolution options
- Radio buttons or list selector
- Options: Accept Ours, Accept Theirs, AI Resolve, Skip
- **Dependencies:** Task 4.2
- **Deliverables:** Resolution options

### [x] Task 4.4: Wire resolution keybindings
- R → Accept ours
- S → Accept theirs (when not in memory browser)
- O → AI resolve
- T → Skip
- **Dependencies:** Task 4.3
- **Deliverables:** Keybindings

### [ ] Task 4.5: Emit resolution event
- On selection, emit ConductorEvent::ConflictResolution
- Include file and resolution method
- **Dependencies:** Task 4.4
- **Deliverables:** Event emission

---

## Phase 5: Parallel ConductorEvents (4 tasks)

### [x] Task 5.1: Add parallel event variants (1-6)
- ParallelStarted, WorkerStatusChanged, MergeQueueUpdated
- ConflictDetected, ConflictResolved, ParallelCompleted
- **Dependencies:** None
- **Deliverables:** Event variants 1-6

### [x] Task 5.2: Add parallel event variants (7-12)
- ParallelPaused, ParallelResumed, WorkerOutput
- WorkerError, MergeProgress, MergeConflict
- **Dependencies:** Task 5.1
- **Deliverables:** Event variants 7-12

### [~] Task 5.3: Add event serialization
- Ensure all new events serialize/deserialize
- Add to event parsing in polling.rs
- **Dependencies:** Task 5.2
- **Deliverables:** Serialization

### [~] Task 5.4: Add event tests
- Test serialization for all 12 events
- Test event parsing
- **Dependencies:** Task 5.3
- **Deliverables:** Tests

---

## Phase 6: Header/Footer/Dashboard Updates (3 tasks)

### [ ] Task 6.1: Update header for parallel mode
- Show parallel status when active
- Show worker count
- **Dependencies:** Phase 3
- **Deliverables:** Header update

### [ ] Task 6.2: Update dashboard for parallel metrics
- Show worker status breakdown
- Show merge queue length
- Show conflict count
- **Dependencies:** Task 6.1
- **Deliverables:** Dashboard update

### [ ] Task 6.3: Update footer key hints
- Add parallel-specific key hints
- Context-aware: show when parallel active
- **Dependencies:** Task 6.2
- **Deliverables:** Footer update

---

## Phase 7: Final Verification (2 tasks)

### [ ] Task 7.1: Integration testing
- Test parallel view with mock data
- Test conflict panel workflow
- Test all keybindings
- **Dependencies:** All previous
- **Deliverables:** Integration tests

### [ ] Task 7.2: Tzar Review
- Code review
- Check state machine correctness
- Verify test coverage
- **Dependencies:** Task 7.1
- **Deliverables:** Tzar report

---

## Total Tasks: 27

**Estimated Duration:** 4-5 days

**Dependencies:** 01-conductor-phase1-ui-primitives

**Blocks:** 05-conductor-phase5-parallel-polling
