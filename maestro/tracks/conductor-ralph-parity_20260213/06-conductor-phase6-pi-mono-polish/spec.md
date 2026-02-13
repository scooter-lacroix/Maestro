# Subtrack 06: Pi-Mono Deep Integration + Polish - Specification

**Track ID:** 06-conductor-phase6-pi-mono-polish
**Parent:** conductor-ralph-parity_20260213
**Phase:** 6
**Status:** New
**Dependencies:** 01, 02, 03

---

## Objective

Add Pi-Mono agent start options, display Pi-Mono detection status, add memory stats to dashboard, enable iteration timing display, and add `maestro implement` launcher from TUI.

---

## Requirements

### R1: Pi-Mono Start Options
- **R1.1:** Add `--pi-agent <name>` to start command builder
- **R1.2:** Add `--pi-chain <agent1,agent2,...>` to start command builder
- **R1.3:** Add `--pi-parallel <agent1,agent2,...>` to start command builder
- **R1.4:** Create Pi-Mono mode selector modal

### R2: Pi-Mono Detection Status
- **R2.1:** Poll `pi --version` to detect Pi-Mono availability
- **R2.2:** Show Pi-Mono status in dashboard
- **R2.3:** Show available agents list
- **R2.4:** Disable Pi-Mono options if not available

### R3: Memory Stats in Dashboard
- **R3.1:** Query `MemoryService::get_stats()` for total memories
- **R3.2:** Show breakdown by category
- **R3.3:** Show memory storage size (if available)
- **R3.4:** Update periodically

### R4: Iteration Timing Display
- **R4.1:** Add `duration_ms` field to IterationLog
- **R4.2:** Show iteration duration in history list
- **R4.3:** Format as "1m 30s" or "45.2s"
- **R4.4:** Show average iteration time in dashboard

### R5: Iteration Drill-Down
- **R5.1:** Enable click/Enter on iteration history item
- **R5.2:** Show full iteration details in details panel
- **R5.3:** Show output for selected iteration
- **R5.4:** Show prompt used for selected iteration

### R6: Subagent Tree Wiring
- **R6.1:** Populate subagent tree from engine events
- **R6.2:** Show active subagents with status
- **R6.3:** Link to OMP agent status as fallback
- **R6.4:** Show subagent output on selection

### R7: Maestro Implement Launcher
- **R7.1:** Add `I` keybinding to spawn `maestro implement <track>`
- **R7.2:** Show track selector modal
- **R7.3:** Display implementation progress
- **R7.4:** Handle completion notification

---

## Acceptance Criteria

- [ ] Pi-Mono start options appear in command builder
- [ ] Pi-Mono detection shows in dashboard
- [ ] Memory stats visible in dashboard
- [ ] Iteration timing shown in history
- [ ] Click-to-view iteration works
- [ ] Subagent tree populated
- [ ] `I` key spawns maestro implement

---

## Dependencies

- 01-conductor-phase1-ui-primitives (modals)
- 02-conductor-phase2-task-display (details panel)
- 03-conductor-phase3-memory-integration (memory service)

## Blocks

- None
