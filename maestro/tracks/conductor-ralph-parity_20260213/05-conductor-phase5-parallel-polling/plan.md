# Subtrack 05: Parallel Event Polling - Implementation Plan

**Track:** 05-conductor-phase5-parallel-polling
**Parent:** conductor-ralph-parity_20260213
**Status:** New

---

## Phase 1: Parallel EngineEvents (5 tasks)

### [ ] Task 1.1: Add ParallelStarted event
- Add to EngineEvent enum in orchestrate/control.rs
- Fields: group_id, worker_count
- Add serialization
- **Dependencies:** 04-conductor-phase4-parallel-ui
- **Deliverables:** ParallelStarted event

### [ ] Task 1.2: Add ParallelWorkerStatus event
- Fields: worker_id, status, progress
- Add serialization
- **Dependencies:** Task 1.1
- **Deliverables:** ParallelWorkerStatus event

### [ ] Task 1.3: Add merge-related events
- ParallelMergeQueued, ParallelMerging, ParallelConflict, ParallelResolved
- Add serialization for all
- **Dependencies:** Task 1.2
- **Deliverables:** Merge events

### [ ] Task 1.4: Add completion/resume events
- ParallelCompleted, ParallelPaused, ParallelResumed
- Add serialization
- **Dependencies:** Task 1.3
- **Deliverables:** Completion events

### [ ] Task 1.5: Add event tests
- Test serialization for all 9 events
- Test deserialization
- **Dependencies:** Task 1.4
- **Deliverables:** Event tests

---

## Phase 2: Parallel ControlCommands (3 tasks)

### [ ] Task 2.1: Add PauseParallel command
- Add to ControlCommand enum
- Field: group_id
- Add serialization
- **Dependencies:** None
- **Deliverables:** PauseParallel command

### [ ] Task 2.2: Add ResumeParallel command
- Add to ControlCommand enum
- Field: group_id
- Add serialization
- **Dependencies:** Task 2.1
- **Deliverables:** ResumeParallel command

### [ ] Task 2.3: Add ResolveConflict command
- Fields: file, method
- Method options: "ours", "theirs", "ai", "skip"
- Add serialization
- **Dependencies:** Task 2.2
- **Deliverables:** ResolveConflict command

---

## Phase 3: Polling Extension (4 tasks)

### [ ] Task 3.1: Extend process_engine_event()
- Add match arms for parallel EngineEvents
- Map to ConductorEvents
- **Dependencies:** Phase 1
- **Deliverables:** Event mapping

### [ ] Task 3.2: Update ConductorState for parallel
- Add parallel_group field
- Add parallel_workers HashMap
- Add merge_queue field
- **Dependencies:** Task 3.1
- **Deliverables:** State updates

### [ ] Task 3.3: Handle parallel event processing
- Update worker status on ParallelWorkerStatus
- Update merge queue on ParallelMergeQueued
- Trigger conflict panel on ParallelConflict
- **Dependencies:** Task 3.2
- **Deliverables:** Event handling

### [ ] Task 3.4: Add event throttling
- Throttle rapid ParallelWorkerStatus events
- Keep last N events per worker
- **Dependencies:** Task 3.3
- **Deliverables:** Throttling

---

## Phase 4: State Machine Transitions (3 tasks)

### [ ] Task 4.1: Add ParallelRunning transition
- From Running to ParallelRunning
- Update status display
- **Dependencies:** Phase 3
- **Deliverables:** ParallelRunning state

### [ ] Task 4.2: Add ParallelPaused transition
- Handle PauseParallel command
- Update state machine
- **Dependencies:** Task 4.1
- **Deliverables:** ParallelPaused state

### [ ] Task 4.3: Add ParallelMerging transition
- Transition when merge queue starts
- Handle merge completion
- **Dependencies:** Task 4.2
- **Deliverables:** ParallelMerging state

---

## Phase 5: TelemetryBus Broadcasting (3 tasks)

### [ ] Task 5.1: Verify parallel event broadcast
- Ensure all parallel events go to TelemetryBus
- Test subscriber receives events
- **Dependencies:** Phase 1
- **Deliverables:** Broadcast verification

### [ ] Task 5.2: Add event ordering test
- Verify events arrive in order
- Test with rapid events
- **Dependencies:** Task 5.1
- **Deliverables:** Ordering test

### [ ] Task 5.3: Add broadcast tests
- Test all 9 parallel events broadcast
- Test multiple subscribers
- **Dependencies:** Task 5.2
- **Deliverables:** Broadcast tests

---

## Phase 6: Final Verification (2 tasks)

### [ ] Task 6.1: Integration testing
- Test full parallel event flow
- Engine → events.jsonl → polling → ConductorEvent
- **Dependencies:** All previous
- **Deliverables:** Integration tests

### [ ] Task 6.2: Tzar Review
- Code review
- Verify event ordering guarantees
- Check error handling
- **Dependencies:** Task 6.1
- **Deliverables:** Tzar report

---

## Total Tasks: 20

**Estimated Duration:** 2-3 days

**Dependencies:** 04-conductor-phase4-parallel-ui

**Blocks:** None
