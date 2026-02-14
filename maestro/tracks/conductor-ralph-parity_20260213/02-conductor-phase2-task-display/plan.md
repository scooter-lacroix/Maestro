# Subtrack 02: Task Display & Details - Implementation Plan

**Track:** 02-conductor-phase2-task-display
**Parent:** conductor-ralph-parity_20260213
**Status:** New

---

## Phase 1: Blocked/Actionable Indicators (4 tasks)

### [x] Task 1.1: Analyze task dependency determination logic
- Review how dependencies are parsed from plan.md
- Determine where blocked/actionable status is calculated
- Document the data flow
- **Dependencies:** None
- **Deliverables:** Analysis document (analysis-dependency-logic.md)

### [x] Task 1.2: Wire STATUS_BLOCKED in track_tree.rs
- Add logic to determine if task is blocked
- Use `STATUS_BLOCKED` constant from theme.rs
- Test blocked indicator rendering
- **Dependencies:** Task 1.1
- **Deliverables:** Blocked indicator in tree (IMPLEMENTED)

### [x] Task 1.3: Wire STATUS_ACTIONABLE in track_tree.rs
- Add logic to determine if task is actionable (deps met, not started)
- Use `STATUS_ACTIONABLE` constant
- Test actionable indicator rendering
- **Dependencies:** Task 1.2
- **Deliverables:** Actionable indicator in tree (IMPLEMENTED)

### [x] Task 1.4: Add unit tests for status determination
- Test blocked detection with incomplete dependencies
- Test actionable detection with completed dependencies
- Test edge cases (no deps, circular deps)
- **Dependencies:** Task 1.3
- **Deliverables:** Unit tests (PASSED)

---

## Phase 2: Task Dependencies Display (4 tasks)

### [~] Task 2.1: Extend details_panel.rs for dependencies
- Add section for task dependencies
- Format as list with status icons
- **Dependencies:** None
- **Deliverables:** Dependencies section

### [ ] Task 2.2: Add dependency status icons
- ✓ for completed dependencies
- ○ for pending dependencies
- ⊘ for blocked dependencies
- Test icon rendering
- **Dependencies:** Task 2.1
- **Deliverables:** Status icons

### [x] Task 2.2: Add dependency status icons
- Make dependencies clickable
- Navigate to dependency task on click/Enter
- **Dependencies:** Task 2.2
- **Deliverables:** Navigation feature

### [ ] Task 2.4: Add tests for dependency display
- Test dependency list rendering
- Test icon correctness
- Test navigation
- **Dependencies:** Task 2.3
- **Deliverables:** Tests

---

## Phase 3: Description and Notes Display (3 tasks)

### [ ] Task 3.1: Add task.description to details view
- Show description below task title
- Handle multi-line with text wrapping
- Test with various description lengths
- **Dependencies:** None
- **Deliverables:** Description display

### [ ] Task 3.2: Add task.notes to details view
- Show notes section when present
- Format SKIPPED with icon
- Format RATE_LIMITED with backoff info
- **Dependencies:** Task 3.1
- **Deliverables:** Notes display

### [ ] Task 3.3: Add tests for description/notes
- Test description rendering
- Test notes formatting
- Test missing fields handling
- **Dependencies:** Task 3.2
- **Deliverables:** Tests

---

## Phase 4: Real Prompt Preview (4 tasks)

### [ ] Task 4.1: Integrate PromptBuilder in details view
- Import PromptBuilder from orchestrate module
- Call build_prompt() for current task
- **Dependencies:** None
- **Deliverables:** PromptBuilder integration

### [ ] Task 4.2: Render actual prompt in Alt+3 view
- Replace placeholder text with real prompt
- Add scrolling for long prompts
- Test with various task types
- **Dependencies:** Task 4.1
- **Deliverables:** Real prompt display

### [ ] Task 4.3: Add token count estimate
- Estimate tokens in prompt (chars / 4 approximation)
- Show token count in header
- **Dependencies:** Task 4.2
- **Deliverables:** Token count

### [ ] Task 4.4: Implement prompt caching
- Cache built prompt until task changes
- Invalidate cache on task switch
- Add cache hit/miss logging
- **Dependencies:** Task 4.3
- **Deliverables:** Prompt caching

---

## Phase 5: Task Breakdown in Dashboard (4 tasks)

### [ ] Task 5.1: Calculate task breakdown counts
- Count completed tasks
- Count actionable tasks
- Count blocked tasks
- **Dependencies:** None
- **Deliverables:** Breakdown calculation

### [ ] Task 5.2: Add breakdown section to dashboard
- Add new section to dashboard overlay
- Show ✓/○/⊘ counts with labels
- **Dependencies:** Task 5.1
- **Deliverables:** Dashboard section

### [ ] Task 5.3: Add percentage progress bar
- Calculate (done / total) * 100
- Render visual progress bar
- Show percentage text
- **Dependencies:** Task 5.2
- **Deliverables:** Progress bar

### [ ] Task 5.4: Add tests for dashboard breakdown
- Test count calculations
- Test progress bar rendering
- Test edge cases (0 tasks, all done)
- **Dependencies:** Task 5.3
- **Deliverables:** Tests

---

## Phase 6: Final Verification (2 tasks)

### [ ] Task 6.1: Integration testing
- Test all display improvements together
- Verify no regressions
- Test with real track data
- **Dependencies:** All previous
- **Deliverables:** Integration tests

### [ ] Task 6.2: Tzar Review
- Code review
- Verify test coverage
- Check for edge cases
- **Dependencies:** Task 6.1
- **Deliverables:** Tzar report

---

## Total Tasks: 21

**Estimated Duration:** 2-3 days

**Dependencies:** 01-conductor-phase1-ui-primitives

**Blocks:** None
