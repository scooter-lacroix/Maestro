# Subtrack 02: Task Display & Details - Specification

**Track ID:** 02-conductor-phase2-task-display
**Parent:** conductor-ralph-parity_20260213
**Phase:** 2
**Status:** New
**Dependencies:** 01-conductor-phase1-ui-primitives

---

## Objective

Wire blocked (⊘) and actionable (○) indicators in the track tree, display task dependencies and descriptions in details panel, and show real prompt previews.

---

## Requirements

### R1: Blocked/Actionable Indicators in Track Tree
- **R1.1:** Wire `STATUS_BLOCKED` (⊘) indicator in `track_tree.rs`
- **R1.2:** Wire `STATUS_ACTIONABLE` (○) indicator in `track_tree.rs`
- **R1.3:** Determine blocked status from task dependencies
- **R1.4:** Determine actionable status from dependencies met but not started

### R2: Task Dependencies Display
- **R2.1:** Show task dependencies in `details_panel.rs`
- **R2.2:** Use ✓ for completed dependencies
- **R2.3:** Use ○ for pending dependencies
- **R2.4:** Use ⊘ for blocked dependencies

### R3: Task Description Display
- **R3.1:** Display `task.description` in Task details view
- **R3.2:** Handle multi-line descriptions with wrapping
- **R3.3:** Show description below task title

### R4: Task Notes Display
- **R4.1:** Display `task.notes` field
- **R4.2:** Show SKIPPED reason with icon
- **R4.3:** Show RATE_LIMITED reason with backoff info

### R5: Real Prompt Preview
- **R5.1:** Call `PromptBuilder::build_prompt()` for current task
- **R5.2:** Render actual prompt in Alt+3 view
- **R5.3:** Show token count estimate
- **R5.4:** Cache prompt to avoid rebuilding on every frame

### R6: Task Breakdown in Dashboard
- **R6.1:** Add task breakdown to dashboard overlay
- **R6.2:** Show ✓ done count
- **R6.3:** Show ○ actionable count
- **R6.4:** Show ⊘ blocked count
- **R6.5:** Show percentage progress

---

## Acceptance Criteria

- [ ] Blocked tasks show ⊘ in track tree
- [ ] Actionable tasks show ○ in track tree
- [ ] Task details show dependencies with status icons
- [ ] Task description and notes visible in details
- [ ] Alt+3 shows actual prompt, not placeholder
- [ ] Dashboard shows task breakdown

---

## Dependencies

- 01-conductor-phase1-ui-primitives (input modal for potential future use)

## Blocks

- None (standalone display improvements)
