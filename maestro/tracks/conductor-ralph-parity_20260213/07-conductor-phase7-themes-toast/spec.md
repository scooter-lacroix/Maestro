# Subtrack 07: Theme System + Toast - Specification

**Track ID:** 07-conductor-phase7-themes-toast
**Parent:** conductor-ralph-parity_20260213
**Phase:** 7
**Status:** New
**Dependencies:** 01-conductor-phase1-ui-primitives

---

## Objective

Add 4 additional themes (catppuccin, dracula, high-contrast, solarized-light), implement theme selector modal, and add toast notification system with auto-dismiss.

---

## Requirements

### R1: Additional Theme Palettes
- **R1.1:** Add Catppuccin theme (mocha flavor)
- **R1.2:** Add Dracula theme
- **R1.3:** Add High Contrast theme (accessibility)
- **R1.4:** Add Solarized Light theme
- **R1.5:** Each theme defines all semantic colors

### R2: Theme Selector
- **R2.1:** Implement `Theme::from_name(name: &str) -> Option<Theme>`
- **R2.2:** Add `T` keybinding to open theme selector
- **R2.3:** Preview theme on hover/selection
- **R2.4:** Apply theme immediately on selection

### R3: Theme Persistence
- **R3.1:** Save theme preference to `~/.maestro/conductor.toml`
- **R3.2:** Load theme on startup
- **R3.3:** Default to Tokyo Night if no preference

### R4: ToastQueue Implementation
- **R4.1:** Create `ToastQueue` struct with VecDeque
- **R4.2:** Add `ToastLevel` enum (Info, Warning, Error, Success)
- **R4.3:** Each toast has: message, level, created_at, duration
- **R4.4:** Auto-dismiss after duration (default 5s)

### R5: Toast Rendering
- **R5.1:** Render toasts as overlay in bottom-right
- **R5.2:** Stack multiple toasts vertically
- **R5.3:** Color by level (info=blue, warning=yellow, error=red, success=green)
- **R5.4:** Animate entrance/exit (fade in/out)

### R6: Replace StatusMessage with Toast
- **R6.1:** Replace `StatusMessage` emissions with `toast_queue.push()`
- **R6.2:** Identify all StatusMessage usage points
- **R6.3:** Convert to appropriate toast levels
- **R6.4:** Remove old StatusMessage code

---

## Acceptance Criteria

- [ ] 5 themes available (Tokyo Night + 4 new)
- [ ] `T` key opens theme selector
- [ ] Theme persists across sessions
- [ ] ToastQueue displays toasts
- [ ] Auto-dismiss after 5 seconds
- [ ] All StatusMessage replaced with toasts

---

## Dependencies

- 01-conductor-phase1-ui-primitives (SelectorModal)

## Blocks

- None
