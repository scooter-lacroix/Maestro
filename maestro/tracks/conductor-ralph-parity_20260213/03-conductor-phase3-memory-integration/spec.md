# Subtrack 03: Memory System Integration - Specification

**Track ID:** 03-conductor-phase3-memory-integration
**Parent:** conductor-ralph-parity_20260213
**Phase:** 3
**Status:** New
**Dependencies:** 01-conductor-phase1-ui-primitives

---

## Objective

Integrate MemoryService into ConductorPane, add memory browser overlay, and enable store/search/list/delete memory operations from the TUI.

---

## Requirements

### R1: MemoryService Integration
- **R1.1:** Add `MemoryService` reference to ConductorPane
- **R1.2:** Initialize memory service from Maestro config
- **R1.3:** Handle connection errors gracefully

### R2: Memory Browser Overlay
- **R2.1:** Create `memory_browser.rs` component
- **R2.2:** List memories with category icons
- **R2.3:** Add search input field
- **R2.4:** Add category filter (dropdown or tabs)
- **R2.5:** Pagination for >100 memories

### R3: Store Memory Modal
- **R3.1:** Reuse InputModal for memory content
- **R3.2:** Add category selector
- **R3.3:** Call `MemoryService::store_memory()` on submit
- **R3.4:** Show success/error toast

### R4: Memory Search
- **R4.1:** Implement search input in browser
- **R4.2:** Call `MemoryService::search_memories()` with query
- **R4.3:** Display search results
- **R4.4:** Clear search to show all

### R5: Memory List View
- **R5.1:** Show memory list with content preview
- **R5.2:** Show category, created_at, expires_at
- **R5.3:** Sort by created_at descending

### R6: Delete Memory
- **R6.1:** Add delete keybinding (d) in memory browser
- **R6.2:** Show confirmation dialog
- **R6.3:** Call `MemoryService::delete_memory()` on confirm
- **R6.4:** Show success toast

### R7: Keybindings
- **R7.1:** `m` → Open memory browser
- **R7.2:** `n` (or `Shift+M`) → Open store memory modal
- **R7.3:** `d` → Delete selected memory (in browser)
- **R7.4:** `/` → Focus search input (in browser)
- **R7.5:** Update help text

---

## Acceptance Criteria

- [ ] Memory browser opens with `m` key
- [ ] Memories listed with category icons
- [ ] Search filters memories by content
- [ ] Category filter works
- [ ] Store memory modal creates new memory
- [ ] Delete removes memory with confirmation
- [ ] All keybindings functional

---

## Dependencies

- 01-conductor-phase1-ui-primitives (InputModal, SelectorModal)

## Blocks

- 06-conductor-phase6-pi-mono-polish
