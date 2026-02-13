# Subtrack 07: Theme System + Toast - Implementation Plan

**Track:** 07-conductor-phase7-themes-toast
**Parent:** conductor-ralph-parity_20260213
**Status:** New

---

## Phase 1: Additional Theme Palettes (5 tasks)

### [ ] Task 1.1: Add Catppuccin theme
- Define ConductorTheme with Catppuccin Mocha colors
- Map all semantic colors (bg, fg, status, task, accent)
- Add test for theme construction
- **Dependencies:** None
- **Deliverables:** Catppuccin theme

### [ ] Task 1.2: Add Dracula theme
- Define ConductorTheme with Dracula colors
- Map all semantic colors
- Add test
- **Dependencies:** Task 1.1
- **Deliverables:** Dracula theme

### [ ] Task 1.3: Add High Contrast theme
- Define with maximum contrast (black/white)
- Ensure accessibility compliance
- Add test
- **Dependencies:** Task 1.2
- **Deliverables:** High Contrast theme

### [ ] Task 1.4: Add Solarized Light theme
- Define with Solarized Light palette
- Light background theme
- Add test
- **Dependencies:** Task 1.3
- **Deliverables:** Solarized Light theme

### [ ] Task 1.5: Create theme registry
- HashMap<String, ConductorTheme>
- Register all 5 themes
- Add get_theme(name) function
- **Dependencies:** Task 1.4
- **Deliverables:** Theme registry

---

## Phase 2: Theme Selector (4 tasks)

### [ ] Task 2.1: Implement Theme::from_name()
- Lookup in registry
- Return Option<ConductorTheme>
- Add tests for all themes
- **Dependencies:** Phase 1
- **Deliverables:** from_name method

### [ ] Task 2.2: Add T keybinding
- Open theme selector modal
- Reuse SelectorModal
- **Dependencies:** 01-conductor-phase1-ui-primitives
- **Deliverables:** T keybinding

### [ ] Task 2.3: Implement theme preview
- On selection change, preview theme
- Use temporary theme state
- **Dependencies:** Task 2.2
- **Deliverables:** Theme preview

### [ ] Task 2.4: Apply theme on confirm
- Update ConductorState.theme
- Trigger re-render
- **Dependencies:** Task 2.3
- **Deliverables:** Theme application

---

## Phase 3: Theme Persistence (3 tasks)

### [ ] Task 3.1: Save theme to conductor.toml
- On theme change, write to config file
- Use TOML format
- **Dependencies:** Phase 2
- **Deliverables:** Save to config

### [ ] Task 3.2: Load theme on startup
- Read conductor.toml
- Apply saved theme or default
- **Dependencies:** Task 3.1
- **Deliverables:** Load from config

### [ ] Task 3.3: Add config migration
- Handle missing config file
- Add theme field if not present
- **Dependencies:** Task 3.2
- **Deliverables:** Config migration

---

## Phase 4: ToastQueue Implementation (4 tasks)

### [ ] Task 4.1: Define Toast struct
- message: String
- level: ToastLevel
- created_at: Instant
- duration: Duration
- **Dependencies:** None
- **Deliverables:** Toast struct

### [ ] Task 4.2: Define ToastLevel enum
- Info, Warning, Error, Success
- Add color mapping
- **Dependencies:** Task 4.1
- **Deliverables:** ToastLevel enum

### [ ] Task 4.3: Create ToastQueue struct
- toasts: VecDeque<Toast>
- push(), pop(), iter_expired() methods
- max_capacity (10)
- **Dependencies:** Task 4.2
- **Deliverables:** ToastQueue struct

### [ ] Task 4.4: Add auto-dismiss logic
- Check elapsed time on each render
- Remove expired toasts
- Add unit tests
- **Dependencies:** Task 4.3
- **Deliverables:** Auto-dismiss

---

## Phase 5: Toast Rendering (4 tasks)

### [ ] Task 5.1: Render toast overlay
- Bottom-right corner
- Fixed width (50 chars)
- Stack vertically
- **Dependencies:** Phase 4
- **Deliverables:** Toast rendering

### [ ] Task 5.2: Add level colors
- Info = blue border
- Warning = yellow border
- Error = red border
- Success = green border
- **Dependencies:** Task 5.1
- **Deliverables:** Level colors

### [ ] Task 5.3: Add fade animation
- Fade in on push (500ms)
- Fade out before dismiss (500ms)
- Use alpha blending
- **Dependencies:** Task 5.2
- **Deliverables:** Fade animation

### [ ] Task 5.4: Add toast tests
- Test rendering
- Test stacking
- Test expiration
- **Dependencies:** Task 5.3
- **Deliverables:** Tests

---

## Phase 6: Replace StatusMessage (3 tasks)

### [ ] Task 6.1: Identify StatusMessage usage
- Grep for StatusMessage
- Document all usage points
- **Dependencies:** None
- **Deliverables:** Usage inventory

### [ ] Task 6.2: Convert to toast_queue.push()
- Replace each StatusMessage
- Map to appropriate ToastLevel
- **Dependencies:** Task 6.1, Phase 4
- **Deliverables:** Conversion

### [ ] Task 6.3: Remove StatusMessage code
- Delete unused StatusMessage struct
- Clean up imports
- **Dependencies:** Task 6.2
- **Deliverables:** Code cleanup

---

## Phase 7: Final Verification (2 tasks)

### [ ] Task 7.1: Integration testing
- Test all themes
- Test theme persistence
- Test toast notifications
- Test auto-dismiss
- **Dependencies:** All previous
- **Deliverables:** Integration tests

### [ ] Task 7.2: Tzar Review
- Code review
- Verify accessibility (high contrast)
- Check theme color accuracy
- **Dependencies:** Task 7.1
- **Deliverables:** Tzar report

---

## Total Tasks: 25

**Estimated Duration:** 2-3 days

**Dependencies:** 01-conductor-phase1-ui-primitives

**Blocks:** None
