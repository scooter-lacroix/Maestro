# Subtrack 06: Pi-Mono Deep Integration + Polish - Implementation Plan

**Track:** 06-conductor-phase6-pi-mono-polish
**Parent:** conductor-ralph-parity_20260213
**Status:** New

---

## Phase 1: Pi-Mono Start Options (4 tasks)

### [ ] Task 1.1: Extend command builder for --pi-agent
- Add `pi_agent: Option<String>` to StartCommandOptions
- Append `--pi-agent <name>` to command string
- **Dependencies:** None
- **Deliverables:** pi-agent option

### [ ] Task 1.2: Extend command builder for --pi-chain
- Add `pi_chain: Option<Vec<String>>` to options
- Append `--pi-chain agent1,agent2,...`
- **Dependencies:** Task 1.1
- **Deliverables:** pi-chain option

### [ ] Task 1.3: Extend command builder for --pi-parallel
- Add `pi_parallel: Option<Vec<String>>` to options
- Append `--pi-parallel agent1,agent2,...`
- **Dependencies:** Task 1.2
- **Deliverables:** pi-parallel option

### [ ] Task 1.4: Create Pi-Mono mode selector modal
- Reuse SelectorModal
- Options: Single, Chain, Parallel
- Show agent input after mode selection
- **Dependencies:** 01-conductor-phase1-ui-primitives
- **Deliverables:** Mode selector modal

---

## Phase 2: Pi-Mono Detection Status (4 tasks)

### [ ] Task 2.1: Implement Pi-Mono detection
- Run `pi --version` on startup
- Cache availability status
- Retry periodically
- **Dependencies:** None
- **Deliverables:** Detection logic

### [ ] Task 2.2: Show status in dashboard
- Add "Pi-Mono: Available/Not Found" line
- Green checkmark or red X
- **Dependencies:** Task 2.1
- **Deliverables:** Dashboard status

### [ ] Task 2.3: Query available agents
- Run `pi list-agents` or equivalent
- Cache agent list
- **Dependencies:** Task 2.2
- **Deliverables:** Agent list

### [ ] Task 2.4: Disable Pi-Mono options when unavailable
- Gray out Pi-Mono menu options
- Show tooltip "Pi-Mono not installed"
- **Dependencies:** Task 2.3
- **Deliverables:** Conditional UI

---

## Phase 3: Memory Stats in Dashboard (3 tasks)

### [ ] Task 3.1: Add get_stats() to MemoryService
- Return total count, by category, storage size
- **Dependencies:** 03-conductor-phase3-memory-integration
- **Deliverables:** get_stats method

### [ ] Task 3.2: Poll memory stats periodically
- Query every 30 seconds
- Store in ConductorState
- **Dependencies:** Task 3.1
- **Deliverables:** Stats polling

### [ ] Task 3.3: Render memory stats in dashboard
- Show "Memories: X total"
- Show breakdown by category icons
- **Dependencies:** Task 3.2
- **Deliverables:** Dashboard rendering

---

## Phase 4: Iteration Timing Display (3 tasks)

### [ ] Task 4.1: Add duration_ms to IterationLog
- Record start time on iteration begin
- Calculate duration on iteration end
- Store in log
- **Dependencies:** None
- **Deliverables:** Duration field

### [ ] Task 4.2: Show duration in history list
- Format as "1m 30s" or "45.2s"
- Display next to iteration number
- **Dependencies:** Task 4.1
- **Deliverables:** Duration display

### [ ] Task 4.3: Show average iteration time
- Calculate average over last N iterations
- Display in dashboard
- **Dependencies:** Task 4.2
- **Deliverables:** Average time

---

## Phase 5: Iteration Drill-Down (3 tasks)

### [ ] Task 5.1: Add click handler for iteration history
- Track selected iteration index
- Enable Enter key to select
- **Dependencies:** None
- **Deliverables:** Click handler

### [ ] Task 5.2: Show full iteration details
- Display complete output
- Show prompt used
- Show task details
- **Dependencies:** Task 5.1
- **Deliverables:** Details view

### [ ] Task 5.3: Add tests for drill-down
- Test selection
- Test details display
- **Dependencies:** Task 5.2
- **Deliverables:** Tests

---

## Phase 6: Subagent Tree Wiring (3 tasks)

### [ ] Task 6.1: Populate from engine events
- Handle SubagentStarted/Completed events
- Update tree structure
- **Dependencies:** None
- **Deliverables:** Event handling

### [ ] Task 6.2: Fallback to OMP status
- If no engine events, query OMP
- Show active OMP agents
- **Dependencies:** Task 6.1
- **Deliverables:** OMP fallback

### [ ] Task 6.3: Show subagent output
- On selection, show subagent's output
- Link to iteration log
- **Dependencies:** Task 6.2
- **Deliverables:** Output display

---

## Phase 7: Maestro Implement Launcher (3 tasks)

### [ ] Task 7.1: Add I keybinding
- Open track selector modal
- List available tracks from tracks.md
- **Dependencies:** 01-conductor-phase1-ui-primitives
- **Deliverables:** I keybinding

### [ ] Task 7.2: Spawn maestro implement
- On track selection, spawn process
- Show progress indicator
- **Dependencies:** Task 7.1
- **Deliverables:** Process spawn

### [ ] Task 7.3: Show completion notification
- Display toast on completion
- Handle errors
- **Dependencies:** Task 7.2
- **Deliverables:** Notification

---

## Phase 8: Final Verification (2 tasks)

### [ ] Task 8.1: Integration testing
- Test all Pi-Mono options
- Test memory stats
- Test timing display
- Test implement launcher
- **Dependencies:** All previous
- **Deliverables:** Integration tests

### [ ] Task 8.2: Tzar Review
- Code review
- Verify Pi-Mono integration
- Check error handling
- **Dependencies:** Task 8.1
- **Deliverables:** Tzar report

---

## Total Tasks: 25

**Estimated Duration:** 3-4 days

**Dependencies:** 01, 02, 03

**Blocks:** None
