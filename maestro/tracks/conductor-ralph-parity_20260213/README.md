# Conductor 100% Ralph-TUI Parity - Master Track

**Track ID:** `conductor-ralph-parity_20260213`  
**Type:** Master Orchestration Track  
**Status:** In Progress  
**Created:** 2026-02-13  
**Lead:** oracle (orchestration and final reviews)

---

## Overview

This master track coordinates 7 phase subtracks that bring Maestro Conductor pane to **100% feature parity with Ralph-TUI**, plus Maestro-specific deep integration with CLI tools, memory system, and steering.

**Goal:** Close all 57 gaps identified in the [gap analysis](../../docs/conductor-ralph-integration-gap-analysis.md) to achieve complete Ralph-TUI parity.

**Scope:**
- Conductor pane (`crates/cockpit/src/conductor/`) - 20 files
- Engine control surface (`maestro/leindex/rust/src/orchestrate/control.rs`, `engine.rs`) - 14 files
- 7 individual subtracks (one per implementation phase)

---

## Quick Start

### For Developers

```bash
# Navigate to track directory
cd maestro/tracks/conductor-ralph-parity_20260213

# View master track specification
cat spec.md

# View implementation plan
cat plan.md

# Execute a specific subtrack
/maestro:implement 01-conductor-phase1-ui-primitives
/maestro:implement 02-conductor-phase2-task-display
# ... etc.

# Track overall progress
/maestro:status conductor-ralph-parity
```

### For End Users

The Conductor module will be accessible via Cockpit TUI once all phases are complete:

```bash
# Launch Cockpit TUI
maestro tui

# Navigate to Analysis tab
# Select a track using O key
# Press s to start conductor loop
```

---

## Orchestration

### Agent Swarm Configuration

**Team Leader:** oracle (orchestration and final reviews)

**Active Teammates (4 concurrent):**
- **explore:** Implementation tasks (conductor UI, modals, keybindings)
- **feature-dev:code-architect:** Subtrack architecture and design verification
- **feature-dev:code-reviewer:** Code quality and Tzar reviews
- **plugin-dev:agent-creator:** Subtrack creation and spec/plan generation

**Orchestration Rules:**
- Team leader activates 4 teammates in parallel
- Team leader goes DORMANT after task delegation (ceases all activity including monitoring)
- Running teammates message team leader when they need guidance/new tasks
- Teammates are pruned (shutdown) when their respective stage is complete
- Reviews conducted ONLY after all teammates complete their per-stage assigned tasks

### Execution Phases

The master track executes in 7 phases:

| Phase | Subtrack | Duration | Dependencies | Status |
|--------|-----------|----------|--------------|--------|
| 1. UI Primitives + Steering | [01-conductor-phase1-ui-primitives](./01-conductor-phase1-ui-primitives/) | None | ✅ COMPLETE |
| 2. Task Display & Details | [02-conductor-phase2-task-display](./02-conductor-phase2-task-display/) | Phase 1 | ✅ COMPLETE |
| 3. Memory Integration | [03-conductor-phase3-memory-integration](./03-conductor-phase3-memory-integration/) | Phase 1 | ⏳ IN PROGRESS |
| 4. Parallel UI | [04-conductor-phase4-parallel-ui](./04-conductor-phase4-parallel-ui/) | Phase 1 | ⏳ PLANNED |
| 5. Parallel Polling | [05-conductor-phase5-parallel-polling](./05-conductor-phase5-parallel-polling/) | Phase 4 | ⏳ PLANNED |
| 6. Pi-Mono + Polish | [06-conductor-phase6-pi-mono-polish](./06-conductor-phase6-pi-mono-polish/) | Phases 1-3 | ⏳ PLANNED |
| 7. Themes + Toast | [07-conductor-phase7-themes-toast](./07-conductor-phase7-themes-toast/) | Phase 1 | ⏳ PLANNED |

### Subtrack Orchestration

#### Wave 1: Foundation (No Dependencies)
1. **01-conductor-phase1-ui-primitives** - Sequential, blocks all other phases
   - Deliverables: UI primitives + steering implementation
   - Dependencies: None

#### Wave 2: Parallel Execution (After 01 Complete)
2. **02-conductor-phase2-task-display** - Parallel execution
   - Deliverables: Task display & details
   - Dependencies: Phase 1

3. **03-conductor-phase3-memory-integration** - Parallel execution
   - Deliverables: Memory system integration
   - Dependencies: Phase 1

4. **04-conductor-phase4-parallel-ui** - Parallel execution
   - Deliverables: Parallel execution UI
   - Dependencies: Phase 1

5. **07-conductor-phase7-themes-toast** - Parallel execution
   - Deliverables: Theme system + toast
   - Dependencies: Phase 1

#### Wave 3: Sequential Dependencies
6. **05-conductor-phase5-parallel-polling** - Sequential after Phase 4
   - Deliverables: Parallel event polling
   - Dependencies: Phase 4

#### Wave 4: Final Integration (After 01, 02, 03 Complete)
7. **06-conductor-phase6-pi-mono-polish** - Complex integration
   - Deliverables: Pi-Mono deep integration + polish
   - Dependencies: Phases 1-3

---

## Dependency Graph

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Conductor Ralph Parity Track                    │
│                  (conductor-ralph-parity_20260213)           │
└─────────────────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
        ▼                 ▼                 ▼
  ┌───────────┐    ┌───────────┐    ┌───────────┐
  │  Phase 1  │    │  Phase 2  │    │  Phase 3  │
  │  (Found)  │◄───│  (Display) │    │ (Memory)  │
  │ UI Prims  │    │ Task Dets │    │ Integration │
  └──────┬────┘    └──────┬────┘    └──────┬────┘
         │                  │                 │
         │  ┌───────────────┼─────────────────┐│
         │  │               │                 ││
         │  ▼               ▼                 ▼│
         │  ┌───────────┐  ┌───────────┐  ┌───────────┐
         │  │ Phase 4  │  │ Phase 7  │  │ Phase 5  │
         └──►│ Parallel  │  │ Themes   │  │ Parallel  │
            │ UI        │  │ Toast     │  │ Polling   │
            └──────┬────┘  └───────────┘  └─────┬─────┘
                   │                              │
                   │                              ▼
                   │                    ┌───────────┐
                   └─────────────────────►│  Phase 6  │
                                       │ Pi-Mono   │
                                       │ Polish     │
                                       └───────────┘

Legend:
  ─── Sequential dependency (one-way)
  ───► Blocks (one phase must complete before another)
  ◄─── Parallel execution (phases can run simultaneously)
```

### Dependency Rules

| Phase | Blocks | Blocked By |
|--------|--------|-------------|
| 1 (UI Primitives) | 2, 3, 4, 5, 6, 7 | None (Foundation) |
| 2 (Task Display) | 6 | 1 |
| 3 (Memory) | 6 | 1 |
| 4 (Parallel UI) | 5 | 1 |
| 5 (Parallel Polling) | None | 4 |
| 6 (Pi-Mono) | None | 1, 2, 3 |
| 7 (Themes + Toast) | None | 1 |

**Key Insights:**
- **Phase 1 is the critical foundation** - All other phases depend on it
- **Phases 2, 3, 4, 7 can run in parallel** after Phase 1 completes
- **Phase 5 is sequential** after Phase 4 (requires parallel state types)
- **Phase 6 is final integration** after foundational phases complete

---

## Key Features by Phase

### Phase 1: UI Primitives + Steering ✅
- Input modal (text input overlay)
- Selector modal (list selection overlay)
- Steering message injection (ControlCommand::Steer)
- Runtime configuration (max iterations, mode, agent switching)
- Toggle sandbox/dangerous modes
- **Status:** Complete - Modals implemented, steering functional

### Phase 2: Task Display & Details ✅
- Blocked task indicators (⊘)
- Actionable task indicators (○)
- Task dependencies display with status icons
- Task description and notes in details panel
- Real prompt preview (calls PromptBuilder)
- Task breakdown in dashboard (✓/○/⊘)
- **Status:** Complete - All task display enhancements implemented

### Phase 3: Memory Integration ⏳
- MemoryService integration in ConductorPane
- Memory browser overlay (search, category filters)
- Store memory modal (Shift+M keybinding)
- Search and list memories
- Delete memory capability
- **Status:** In progress

### Phase 4: Parallel UI ⏳
- Parallel state types (ParallelStatus, WorkerStatus, MergeStatus)
- Parallel worker state (worker_id, task_id, status, branch)
- Merge queue tracking (MergeQueueEntry)
- Conflict resolution (ConflictInfo, ConflictFile)
- Parallel view UI (worker list + merge queue)
- Conflict panel (per-file resolution modal)
- **Status:** Planned

### Phase 5: Parallel Polling ⏳
- 9 parallel EngineEvent variants
- 3 parallel ControlCommand variants
- Extend polling.rs for parallel events
- Wire parallel events to state machine transitions
- **Status:** Planned

### Phase 6: Pi-Mono + Polish ⏳
- Pi-Mono start options (--pi-agent, --pi-chain, --pi-parallel)
- Pi-Mono mode selector modal
- Pi-Mono detection status in dashboard
- Memory dashboard stats (total by category)
- Iteration timing in history
- Subagent tree wiring (OMP status or engine events)
- `maestro implement` launcher from TUI
- **Status:** Planned

### Phase 7: Themes + Toast ⏳
- 4 additional themes (catppuccin, dracula, high-contrast, solarized-light)
- Theme::from_name() selector
- Theme selector modal (T keybinding)
- ToastQueue with auto-dismiss
- ToastLevel (Info/Warning/Error)
- Toast notifications overlay (bottom-right)
- **Status:** Planned

---

## Documentation

| Document | Path | Purpose |
|-----------|-------|---------|
| Gap Analysis | [docs/conductor-ralph-integration-gap-analysis.md](../../docs/conductor-ralph-integration-gap-analysis.md) | All 57 gaps to close |
| Implementation Plan | [docs/conductor-implementation-plan.md](../../docs/conductor-implementation-plan.md) | 7-phase breakdown with tasks |
| Master Spec | [spec.md](./spec.md) | Functional requirements (M1-M8) |
| Master Plan | [plan.md](./plan.md) | Detailed implementation tasks per phase |

---

## Testing & Quality

- **Target Coverage:** >98% for all new modules
- **TDD Workflow:** Tests written before implementation
- **Tzar Reviews:** Code quality review before proceeding
- **CI Gates:** All tests must pass before merging

---

## Progress

**Overall Status:** 28% complete (2 of 7 phases)

**Completed:**
- ✅ Phase 1: UI Primitives + Steering
- ✅ Phase 2: Task Display & Details

**In Progress:**
- ⏳ Phase 3: Memory Integration

**Planned:**
- ⏳ Phase 4: Parallel UI
- ⏳ Phase 5: Parallel Polling
- ⏳ Phase 6: Pi-Mono + Polish
- ⏳ Phase 7: Themes + Toast

**Parity Progress:** 42% → **Target: 100%**

---

## Related Resources

- [Ralph TUI](https://github.com/subsy/ralph-tui) - Reference implementation
- [Cockpit Documentation](../../crates/cockpit/src/conductor/README.md) - Conductor pane details
- [Maestro Documentation](../../README.md) - Overall framework docs
