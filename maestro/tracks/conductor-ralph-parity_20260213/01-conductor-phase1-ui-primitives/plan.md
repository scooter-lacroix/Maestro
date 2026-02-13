# Subtrack 01: UI Primitives + Steering - Implementation Plan

**Track:** 01-conductor-phase1-ui-primitives
**Parent:** conductor-ralph-parity_20260213
**Status:** New

---

## Phase 1: Input Modal Component (6 tasks)

### [ ] Task 1.1: Create input_modal.rs module structure
- Create `crates/cockpit/src/conductor/input_modal.rs`
- Define `InputModal` struct with fields: title, prompt, input, cursor_pos, visible
- Add module to `conductor/mod.rs`
- **Dependencies:** None
- **Deliverables:** `input_modal.rs`

### [ ] Task 1.2: Implement InputModal::new() and basic state
- Implement constructor with title and prompt parameters
- Initialize input as empty string, cursor at 0, visible as false
- Add unit test for construction
- **Dependencies:** Task 1.1
- **Deliverables:** Constructor + test

### [ ] Task 1.3: Implement character input handling
- Implement `handle_key(KeyEvent)` for character input
- Insert character at cursor position
- Move cursor forward
- Add tests for: single char, multiple chars, cursor in middle
- **Dependencies:** Task 1.2
- **Deliverables:** `handle_key()` + tests

### [ ] Task 1.4: Implement cursor movement and editing
- Left/Right arrows move cursor
- Home/End jump to start/end
- Backspace deletes before cursor
- Delete deletes at cursor
- Add tests for each operation
- **Dependencies:** Task 1.3
- **Deliverables:** Movement/editing + tests

### [ ] Task 1.5: Implement modal visibility and submit/cancel
- `show()` and `hide()` methods
- Enter returns `Some(input.clone())`
- Esc returns `None`
- Add tests for show/hide/submit/cancel
- **Dependencies:** Task 1.4
- **Deliverables:** Visibility methods + tests

### [ ] Task 1.6: Implement InputModal rendering
- Implement `render(frame, area)` method
- Use `centered_rect(30%, 70%, 20%, 80%)` for position
- Use `Clear` widget before content
- Show border, title, prompt, input with cursor
- Add render test (snapshot if feasible)
- **Dependencies:** Task 1.5
- **Deliverables:** `render()` + test

---

## Phase 2: Selector Modal Component (5 tasks)

### [ ] Task 2.1: Create selector_modal.rs module structure
- Create `crates/cockpit/src/conductor/selector_modal.rs`
- Define generic `SelectorModal<T>` struct
- Fields: title, items (Vec<(String, T)>), selected, visible
- Add module to `conductor/mod.rs`
- **Dependencies:** None
- **Deliverables:** `selector_modal.rs`

### [ ] Task 2.2: Implement SelectorModal construction and state
- Implement `new(title, items)` constructor
- Initialize selected to 0, visible to false
- Add unit test for construction
- **Dependencies:** Task 2.1
- **Deliverables:** Constructor + test

### [ ] Task 2.3: Implement navigation and selection
- Up/Down arrows (and j/k) change selected index
- Wrap around at boundaries
- Enter returns `Some(items[selected].1.clone())`
- Esc returns `None`
- Add tests for navigation, wrap, select, cancel
- **Dependencies:** Task 2.2
- **Deliverables:** Navigation + tests

### [ ] Task 2.4: Implement visibility methods
- `show()` and `hide()` methods
- `set_items()` to update item list
- Add tests for visibility
- **Dependencies:** Task 2.3
- **Deliverables:** Visibility methods + tests

### [ ] Task 2.5: Implement SelectorModal rendering
- Implement `render(frame, area)` method
- Centered overlay with Clear
- Show border, title, items with selection highlight
- Add render test
- **Dependencies:** Task 2.4
- **Deliverables:** `render()` + test

---

## Phase 3: ControlCommand Extensions (5 tasks)

### [ ] Task 3.1: Add Steer command variant
- Add `Steer { message: String }` to `ControlCommand` enum in `orchestrate/control.rs`
- Ensure Serialize/Deserialize derives work
- Add unit test for serialization roundtrip
- **Dependencies:** None
- **Deliverables:** `ControlCommand::Steer`

### [ ] Task 3.2: Add SetMaxIterations and SetMode variants
- Add `SetMaxIterations { max: u64 }`
- Add `SetMode { mode: String }`
- Add serialization tests
- **Dependencies:** Task 3.1
- **Deliverables:** New variants + tests

### [ ] Task 3.3: Add SwitchAgent variant
- Add `SwitchAgent { tool: String, model: Option<String> }`
- Add serialization test
- **Dependencies:** Task 3.2
- **Deliverables:** `ControlCommand::SwitchAgent` + test

### [ ] Task 3.4: Add ToggleSandbox and ToggleDangerous variants
- Add `ToggleSandbox`
- Add `ToggleDangerous`
- Add serialization tests
- **Dependencies:** Task 3.3
- **Deliverables:** Toggle variants + tests

### [ ] Task 3.5: Verify backward compatibility
- Test that existing control.json files parse correctly
- Test that new variants don't break old code paths
- Add regression test
- **Dependencies:** Task 3.4
- **Deliverables:** Compatibility tests

---

## Phase 4: Engine Command Handling (6 tasks)

### [ ] Task 4.1: Add steering message storage to engine
- Add `pending_steering: HashMap<String, String>` to OrchestrateEngine
- Add `check_steering_message()` method
- **Dependencies:** Task 3.1
- **Deliverables:** Steering storage + method

### [ ] Task 4.2: Handle Steer command in control loop
- In engine control loop, read `ControlCommand::Steer`
- Store message in pending_steering for track
- Emit `EngineEvent::SteeringReceived`
- Add test for command handling
- **Dependencies:** Task 4.1
- **Deliverables:** Control loop handler + test

### [ ] Task 4.3: Inject steering into iteration prompt
- In `build_iteration_prompt()`, check for pending steering
- Prepend steering message after task context
- Clear pending steering after injection
- Add test for prompt injection
- **Dependencies:** Task 4.2
- **Deliverables:** Prompt injection + test

### [ ] Task 4.4: Handle SetMaxIterations and SetMode commands
- Read commands in control loop
- Update session config accordingly
- Emit appropriate events
- Add tests
- **Dependencies:** Task 3.2
- **Deliverables:** Command handlers + tests

### [ ] Task 4.5: Handle SwitchAgent command
- Read command in control loop
- Update agent config (tool, model)
- Emit `EngineEvent::AgentSwitched`
- Add test
- **Dependencies:** Task 3.3
- **Deliverables:** SwitchAgent handler + test

### [ ] Task 4.6: Handle ToggleSandbox and ToggleDangerous commands
- Read commands in control loop
- Flip boolean flags in session config
- Emit events
- Add tests
- **Dependencies:** Task 3.4
- **Deliverables:** Toggle handlers + tests

---

## Phase 5: ConductorPane Modal Integration (4 tasks)

### [ ] Task 5.1: Add modal fields to ConductorPane
- Add `steering_modal: Option<InputModal>` field
- Add `selector_modal: Option<SelectorModal<SelectorItem>>` field
- Add `max_iterations_modal: Option<InputModal>` field
- Initialize all as None in constructor
- **Dependencies:** Phase 1, Phase 2
- **Deliverables:** Updated ConductorPane struct

### [ ] Task 5.2: Wire modal key event handling
- In `handle_key_events()`, check if modal is visible
- Route key events to active modal
- Handle modal submit/cancel callbacks
- Write ControlCommand on submit
- **Dependencies:** Task 5.1
- **Deliverables:** Modal key routing

### [ ] Task 5.3: Render modal overlays in pane.rs
- After main content, render active modal
- Use proper z-ordering (modal on top)
- Test rendering with visible modal
- **Dependencies:** Task 5.2
- **Deliverables:** Modal rendering

### [ ] Task 5.4: Update polling.rs for new events
- Handle `EngineEvent::SteeringReceived`
- Handle `EngineEvent::AgentSwitched`
- Handle other new events
- Update state machine transitions
- **Dependencies:** Phase 4
- **Deliverables:** Event handlers

---

## Phase 6: Keybindings (4 tasks)

### [ ] Task 6.1: Add steering keybinding (Ctrl+M)
- Add `Ctrl+m` → open steering modal
- Update `keybindings.rs`
- Add test for keybinding trigger
- **Dependencies:** Task 5.2
- **Deliverables:** Steering keybinding

### [ ] Task 6.2: Add error strategy selector keybinding (e)
- Add `e` → open error strategy selector modal
- Populate selector with Retry/Skip/Abort options
- On select, write `ControlCommand::SetErrorStrategy`
- **Dependencies:** Task 5.2
- **Deliverables:** Error strategy keybinding

### [ ] Task 6.3: Add max iterations keybinding (i)
- Add `i` → open max iterations input modal
- On submit, write `ControlCommand::SetMaxIterations`
- Validate input is numeric
- **Dependencies:** Task 5.2
- **Deliverables:** Max iterations keybinding

### [ ] Task 6.4: Add remaining keybindings
- `m` → Toggle mode (write SetMode)
- `a` → Open agent selector
- `S` (Shift+s) → Toggle sandbox
- `D` (Shift+d) → Toggle dangerous
- Update help text
- **Dependencies:** Task 5.2
- **Deliverables:** All keybindings

---

## Phase 7: Testing & Documentation (3 tasks)

### [ ] Task 7.1: Integration tests for modal workflow
- Test full steering flow: Ctrl+M → type → Enter → command written
- Test selector flow: e → select → command written
- Test cancel flow: open modal → Esc → modal closes
- **Dependencies:** Phase 5, Phase 6
- **Deliverables:** Integration tests

### [ ] Task 7.2: Update conductor-implementation-plan.md
- Mark Phase 1 complete
- Link to this subtrack
- Document new keybindings
- **Dependencies:** All previous
- **Deliverables:** Updated docs

### [ ] Task 7.3: Tzar Review
- Code review for all new code
- Check for security issues (input validation)
- Verify test coverage
- **Dependencies:** All previous
- **Deliverables:** Tzar review report

---

## Total Tasks: 33

**Estimated Duration:** 4-5 days

**Dependencies:** None (foundation subtrack)

**Blocks:** Subtracks 02, 03, 04, 07
