# Conductor Tab Implementation Plan

**Status: Complete ✅** (v2.5)

## Overview

This document outlines the implementation plan for transforming Maestro's "Orchestrate" tab into a fully-functional "Conductor" tab that **fully integrates Ralph TUI** functionality into Maestro.

**Reference Documents:**
- [Conductor-Ralph Mapping](conductor-ralph-mapping.md) - Comprehensive component and data model mapping
- [Ralph ADR](adr/Ralph_Wiggum_as_a_software_engineer.html) - Ralph philosophy and patterns
- [v2.5 Spec](../maestro/tracks/v2-5_20260121/spec.md) - Maestro v2.5 requirements

**Key Principle:** Port Ralph concepts to Rust, don't copy TypeScript code. Leverage existing Maestro infrastructure (`engine.rs`, `runner.rs`, `state.rs`, `parser.rs`) while adding Ralph's rich UI and state management patterns.

## Completed Work

### ✅ Phase 0: Fixed "No tracks found" Bug
- Created `crates/cockpit/src/maestro_paths.rs` for project-aware path resolution.
- Updated `ConductorPane` to use `auto_discover()` for smart path resolution.

### ✅ Phase 1: Renamed Orchestrate → Conductor
- Renamed all components, functions, and keybindings.
- Created `crates/cockpit/src/conductor.rs` as the main entry point.

### ✅ Phase 2: Foundation & Data Model
- Implemented core data models in `crates/cockpit/src/conductor/model.rs`.
- Defined Ralph-inspired theme in `crates/cockpit/src/conductor/theme.rs`.
- Established modular directory structure: `crates/cockpit/src/conductor/`.
- Moved main pane logic to `crates/cockpit/src/conductor/pane.rs`.

### ✅ Phase 3: Core UI Framework
- Created `crates/cockpit/src/conductor/header.rs` for a compact status bar.
- Created `crates/cockpit/src/conductor/footer.rs` for keyboard shortcut display.
- Implemented centralized input handling in `crates/cockpit/src/conductor/keybindings.rs`.
- Refactored `ConductorPane` to use the new 3-tier vertical layout (Header, Main, Footer).

### ✅ Phase 4: Advanced UI Panels & State Refinement
- Implemented hierarchical navigation in `crates/cockpit/src/conductor/track_tree.rs` with j/k support.
- Created `crates/cockpit/src/conductor/details_panel.rs` with [Details|Output|Prompt] mode switching.
- Unified expansion state and selection tracking for both tracks and tasks.
- Applied `ConductorTheme` and Ralph symbols throughout the UI.
- Wired real task counts and statuses into `ConductorState` for the header.
- Added initial `iteration_history.rs` module.

### ✅ Phase 5: State Machine & Live Integration
- Implemented `ConductorState` transition logic in `crates/cockpit/src/conductor/state_machine.rs`.
- Created `crates/cockpit/src/conductor/polling.rs` to monitor `session.json` and `iterations.jsonl`.
- Integrated background polling into the main `App` refresh loop in `crates/cockpit/src/app.rs`.
- Synchronized engine status, iterations, and task metadata with the TUI state.

### ✅ Phase 6: Advanced Ralph Features
- Implemented `crates/cockpit/src/conductor/dashboard.rs` for detailed status overlay.
- Implemented `crates/cockpit/src/conductor/subagent_tree.rs` for tool-call visualization.
- Created `maestro/leindex/rust/src/orchestrate/rate_limit.rs` for engine-side detection.
- Integrated Git status (branch, dirty flag) via `crates/cockpit/src/conductor/git.rs`.
- Added dashboard toggle keybinding ('d') and layout support for subagent hierarchy.

### ✅ Phase 7: Project & Context Enhancement
- Implemented multi-project discovery across all tmux panes in `crates/cockpit/src/maestro_paths.rs`.
- Created `crates/cockpit/src/conductor/project_selector.rs` for visual project switching (`Shift+P`).
- Implemented a 5-phase context engine in `maestro/leindex/rust/src/orchestrate/context.rs`.
- Integrated codebase context injection into the execution loop with smart token budgeting.

### ✅ Phase 8: Pi-Mono Integration (v2.5)
- Integrated Pi-Mono subagent system into Conductor workflows.
- Added model discovery, agent mapping (scout/architect/critic/kraken), and execution engine.
- Configuration wizard and adaptive model selection complete.

---

## Status: Complete ✅

All 8 phases of the Conductor implementation are complete. The Conductor module fully replaces the deprecated Orchestrate pane with Ralph TUI functionality integrated into Maestro.

---

## Technical Guidelines

### Component Communication
- Components should be "dumb" (functional) where possible, taking a reference to `ConductorState` and `ConductorTheme`.
- All user interactions should flow through `ConductorPane::handle_event` or a dedicated `keybindings.rs`.

### State Management
- The `ConductorPane` holds the `ConductorState`.
- State updates occur primarily via `Polling` (external engine changes) or user input.
- Use `std::time::Instant` for elapsed time tracking in the UI to avoid jitter.

### Performance
- Avoid re-parsing `plan.md` every frame. Use the existing `cached_plan` pattern.
- Only tail `iterations.jsonl` from the last known byte offset.

---

## Summary of New File Structure

```
crates/cockpit/src/
├── conductor/
│   ├── mod.rs              # Module exports
│   ├── model.rs            # Data models & Events
│   ├── state_machine.rs    # Transition logic
│   ├── header.rs           # UI: Header
│   ├── footer.rs           # UI: Footer
│   ├── track_tree.rs       # UI: Left Panel
│   ├── details_panel.rs    # UI: Right Panel (Multi-mode)
│   ├── dashboard.rs        # UI: Detailed Status Overlay
│   ├── subagent_tree.rs    # UI: Subagent hierarchy
│   ├── iteration_history.rs # UI: Past iterations
│   ├── theme.rs            # Styling & Symbols
│   ├── keybindings.rs      # Input handling logic
│   └── polling.rs          # Engine integration (polling)
├── conductor.rs            # Main Tab Controller
└── maestro_paths.rs        # Discovery Utilities
```
