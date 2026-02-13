# Subtrack 05: Parallel Event Polling - Specification

**Track ID:** 05-conductor-phase5-parallel-polling
**Parent:** conductor-ralph-parity_20260213
**Phase:** 5
**Status:** New
**Dependencies:** 04-conductor-phase4-parallel-ui

---

## Objective

Extend polling.rs and engine control to handle parallel execution events and commands, enabling real-time parallel status updates in the Conductor.

---

## Requirements

### R1: Parallel EngineEvent Variants
- **R1.1:** Add `EngineEvent::ParallelStarted { group_id: String, worker_count: u32 }`
- **R1.2:** Add `EngineEvent::ParallelWorkerStatus { worker_id: String, status: WorkerStatus, progress: f32 }`
- **R1.3:** Add `EngineEvent::ParallelMergeQueued { worker_id: String, task_id: String }`
- **R1.4:** Add `EngineEvent::ParallelMerging { queue_position: u32, total: u32 }`
- **R1.5:** Add `EngineEvent::ParallelConflict { conflict: ConflictInfo }`
- **R1.6:** Add `EngineEvent::ParallelResolved { file: String, method: String }`
- **R1.7:** Add `EngineEvent::ParallelCompleted { success_count: u32, error_count: u32 }`
- **R1.8:** Add `EngineEvent::ParallelPaused { reason: String }`
- **R1.9:** Add `EngineEvent::ParallelResumed`

### R2: Parallel ControlCommand Variants
- **R2.1:** Add `ControlCommand::PauseParallel { group_id: String }`
- **R2.2:** Add `ControlCommand::ResumeParallel { group_id: String }`
- **R2.3:** Add `ControlCommand::ResolveConflict { file: String, method: String }`

### R3: Polling Extension
- **R3.1:** Extend `process_engine_event()` to map parallel events
- **R3.2:** Convert EngineEvent::Parallel* to ConductorEvent::Parallel*
- **R3.3:** Update parallel state in ConductorState
- **R3.4:** Handle parallel event burst (throttle if needed)

### R4: State Machine Transitions
- **R4.1:** Add ParallelRunning state transition
- **R4.2:** Add ParallelPaused state transition
- **R4.3:** Add ParallelMerging state transition
- **R4.4:** Handle error during parallel execution

### R5: TelemetryBus Broadcasting
- **R5.1:** Broadcast parallel events to all subscribers
- **R5.2:** Include full event payload
- **R5.3:** Maintain event ordering

---

## Acceptance Criteria

- [ ] All 9 parallel EngineEvents emit from engine
- [ ] All 3 parallel ControlCommands write to control.json
- [ ] polling.rs maps parallel events to ConductorEvents
- [ ] State machine transitions for parallel states
- [ ] TelemetryBus broadcasts parallel events

---

## Dependencies

- 04-conductor-phase4-parallel-ui (state types, events)

## Blocks

- None
