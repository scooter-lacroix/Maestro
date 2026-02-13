# Subtrack 03: Memory System Integration - Implementation Plan

**Track:** 03-conductor-phase3-memory-integration
**Parent:** conductor-ralph-parity_20260213
**Status:** New

---

## Phase 1: MemoryService Integration (3 tasks)

### [ ] Task 1.1: Add MemoryService to ConductorPane
- Import MemoryService from memory module
- Add `memory_service: Arc<MemoryService>` field
- Initialize in ConductorPane constructor
- **Dependencies:** None
- **Deliverables:** MemoryService field

### [ ] Task 1.2: Handle connection errors
- Wrap memory calls in error handling
- Display error toast on failure
- Graceful degradation when memory unavailable
- **Dependencies:** Task 1.1
- **Deliverables:** Error handling

### [ ] Task 1.3: Add memory polling
- Poll memories for current track periodically
- Store in `track_memories` HashMap
- **Dependencies:** Task 1.2
- **Deliverables:** Memory polling

---

## Phase 2: Memory Browser Component (5 tasks)

### [ ] Task 2.1: Create memory_browser.rs
- Define MemoryBrowser struct
- Fields: memories, selected, search_query, category_filter, visible
- Add to conductor/mod.rs
- **Dependencies:** None
- **Deliverables:** memory_browser.rs

### [ ] Task 2.2: Implement memory list rendering
- Render list of memories with preview
- Show category icon, created_at, content preview
- Highlight selected item
- **Dependencies:** Task 2.1
- **Deliverables:** List rendering

### [ ] Task 2.3: Implement search functionality
- Add search input field
- Filter memories by query
- Real-time filtering
- **Dependencies:** Task 2.2
- **Deliverables:** Search feature

### [ ] Task 2.4: Implement category filter
- Add category selector (reuse SelectorModal)
- Filter list by selected category
- "All" option shows everything
- **Dependencies:** Task 2.3
- **Deliverables:** Category filter

### [ ] Task 2.5: Implement pagination
- Page size: 50 items
- Next/Prev navigation
- Show page indicator
- **Dependencies:** Task 2.4
- **Deliverables:** Pagination

---

## Phase 3: Store Memory Modal (4 tasks)

### [ ] Task 3.1: Create store memory modal
- Reuse InputModal for content
- Add category selector
- **Dependencies:** 01-conductor-phase1-ui-primitives
- **Deliverables:** Store modal

### [ ] Task 3.2: Wire MemoryService::store_memory()
- On modal submit, call store_memory()
- Include track_id association
- **Dependencies:** Task 3.1, Task 1.1
- **Deliverables:** Store call

### [ ] Task 3.3: Add success/error feedback
- Show toast on success
- Show error toast on failure
- Refresh memory list after store
- **Dependencies:** Task 3.2
- **Deliverables:** Feedback

### [ ] Task 3.4: Add tests for store flow
- Test modal open/close
- Test store call
- Test feedback display
- **Dependencies:** Task 3.3
- **Deliverables:** Tests

---

## Phase 4: Delete Memory (4 tasks)

### [ ] Task 4.1: Add delete keybinding
- `d` key in memory browser
- Show confirmation dialog
- **Dependencies:** Phase 2
- **Deliverables:** Delete keybinding

### [ ] Task 4.2: Implement confirmation dialog
- Reuse SelectorModal for Yes/No
- Show memory content preview in dialog
- **Dependencies:** Task 4.1
- **Deliverables:** Confirmation dialog

### [ ] Task 4.3: Wire MemoryService::delete_memory()
- On confirm, call delete_memory()
- Show success toast
- Refresh list
- **Dependencies:** Task 4.2
- **Deliverables:** Delete call

### [ ] Task 4.4: Add tests for delete flow
- Test keybinding trigger
- Test confirmation
- Test delete call
- **Dependencies:** Task 4.3
- **Deliverables:** Tests

---

## Phase 5: Keybindings and Integration (3 tasks)

### [ ] Task 5.1: Add memory keybindings
- `m` → Open memory browser
- `n` → Open store modal
- `d` → Delete (in browser)
- `/` → Focus search
- **Dependencies:** Phase 2, Phase 3
- **Deliverables:** Keybindings

### [ ] Task 5.2: Update help text
- Add memory section to help overlay
- Document all new keybindings
- **Dependencies:** Task 5.1
- **Deliverables:** Help text

### [ ] Task 5.3: Integration tests
- Test full memory workflow
- Test search/filter
- Test store/delete
- **Dependencies:** All previous
- **Deliverables:** Integration tests

---

## Phase 6: Final Verification (2 tasks)

### [ ] Task 6.1: Performance testing
- Test with 1000+ memories
- Verify pagination performance
- Check memory usage
- **Dependencies:** All previous
- **Deliverables:** Performance report

### [ ] Task 6.2: Tzar Review
- Code review
- Security review (memory data handling)
- Test coverage check
- **Dependencies:** Task 6.1
- **Deliverables:** Tzar report

---

## Total Tasks: 21

**Estimated Duration:** 3-4 days

**Dependencies:** 01-conductor-phase1-ui-primitives

**Blocks:** 06-conductor-phase6-pi-mono-polish
