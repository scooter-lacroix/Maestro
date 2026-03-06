# Subtrack 01: UI Primitives + Steering - Specification

**Track ID:** 01-conductor-phase1-ui-primitives
**Parent:** conductor-ralph-parity_20260213
**Phase:** 1
**Status:** New
**Blocks:** 02, 03, 04, 07

---

## Objective

Implement foundational UI primitives (input modal, selector modal) and steering/runtime configuration control commands. This subtrack is the foundation for all subsequent phases.

---

## Requirements

### R1: Input Modal Component
- **R1.1:** Create `crates/cockpit/src/conductor/input_modal.rs`
- **R1.2:** Support text input with cursor movement (left/right arrows, Home/End)
- **R1.3:** Support character input (alphanumeric, symbols)
- **R1.4:** Support backspace and delete
- **R1.5:** Support Enter to submit and Esc to cancel
- **R1.6:** Render as centered overlay with Clear pattern
- **R1.7:** Configurable title and prompt text

### R2: Selector Modal Component
- **R2.1:** Create `crates/cockpit/src/conductor/selector_modal.rs`
- **R2.2:** Support list navigation (up/down arrows, j/k)
- **R2.3:** Support Enter to select and Esc to cancel
- **R2.4:** Generic over selection type T
- **R2.5:** Render as centered overlay with Clear pattern
- **R2.6:** Configurable title

### R3: ControlCommand Extensions
- **R3.1:** Add `ControlCommand::Steer { message: String }` to `orchestrate/control.rs`
- **R3.2:** Add `ControlCommand::SetMaxIterations { max: u64 }`
- **R3.3:** Add `ControlCommand::SetMode { mode: String }` (plan/build)
- **R3.4:** Add `ControlCommand::SwitchAgent { tool: String, model: Option<String> }`
- **R3.5:** Add `ControlCommand::ToggleSandbox`
- **R3.6:** Add `ControlCommand::ToggleDangerous`
- **R3.7:** Maintain backward compatibility with existing control.json format

### R4: Engine Handling of New Commands
- **R4.1:** Engine reads `Steer` command and stores steering message
- **R4.2:** `build_iteration_prompt()` prepends steering message after task context
- **R4.3:** Engine reads `SetMaxIterations` and updates session config
- **R4.4:** Engine reads `SetMode` and switches mode if valid transition
- **R4.5:** Engine reads `SwitchAgent` and updates agent config
- **R4.6:** Engine reads `ToggleSandbox` and flips sandbox flag
- **R4.7:** Engine reads `ToggleDangerous` and flips dangerous flag
- **R4.8:** All new commands emit appropriate EngineEvent responses

### R5: ConductorPane Modal Integration
- **R5.1:** Add `steering_modal: Option<InputModal>` field to ConductorPane
- **R5.2:** Add `selector_modal: Option<SelectorModal<SelectorItem>>` field
- **R5.3:** Add `max_iterations_modal: Option<InputModal>` field
- **R5.4:** Handle modal input in `handle_key_events()`
- **R5.5:** Render modal overlays in `pane.rs` render function

### R6: Keybindings
- **R6.1:** `Ctrl+M` → Open steering modal
- **R6.2:** `e` → Open error strategy selector
- **R6.3:** `i` → Open max iterations input
- **R6.4:** `m` → Toggle mode (plan/build)
- **R6.5:** `a` → Open agent selector
- **R6.6:** `S` (Shift+s) → Toggle sandbox
- **R6.7:** `D` (Shift+d) → Toggle dangerous
- **R6.8:** Update keybinding help text

### R7: Modal Rendering
- **R7.1:** Modals render centered with `centered_rect(30%, 70%, 20%, 80%)`
- **R7.2:** Use `Clear` widget before rendering modal content
- **R7.3:** Modal has border and title
- **R7.4:** Input modal shows cursor position visually
- **R7.5:** Selector modal highlights selected item

---

## Acceptance Criteria

- [ ] InputModal compiles and passes unit tests
- [ ] SelectorModal compiles and passes unit tests
- [ ] All 6 new ControlCommand variants serialize/deserialize correctly
- [ ] Engine processes Steer command and injects message into prompt
- [ ] Ctrl+M opens steering modal, submit sends Steer command
- [ ] All keybindings functional with existing conductor state
- [ ] No regression in existing conductor tests

---

## Technical Notes

### Steering Message Injection Point

In `build_iteration_prompt()`, after memory context:

```rust
if let Some(steering) = self.check_steering_message(track_id)? {
    prompt.push_str("\n## User Steering Message\n\n");
    prompt.push_str(&steering);
    prompt.push('\n');
}
```

### Modal State Machine

Modals should integrate with the existing ConductorState:
- When modal is open, key events go to modal
- When modal closes, return focus to main conductor
- Modal submission triggers ControlCommand write

---

## Dependencies

- None (this is the foundation subtrack)

## Blocks

- 02-conductor-phase2-task-display
- 03-conductor-phase3-memory-integration
- 04-conductor-phase4-parallel-ui
- 07-conductor-phase7-themes-toast
