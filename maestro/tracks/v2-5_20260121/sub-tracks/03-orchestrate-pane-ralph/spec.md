# Sub-Track 03: Orchestrate Pane (Ralph Port) Integrated into Cockpit - Specification

## Objective

Add a dedicated Orchestrate pane to the Rust Cockpit (Maestro Cockpit v2) by porting and rebranding `subsy/ralph-tui` into Rust/ratatui, integrating the Ralph loop mechanics with Maestro’s track model and LeIndex analysis.

## Inputs (Upstream References)

- `https://github.com/subsy/ralph-tui` (MIT)
- `https://github.com/ghuntley/how-to-ralph-wiggum` (playbook + loop mechanics)

## Requirements

### R1: UX Parity (Look & Feel)

- Preserve the “general” Ralph layout:
  - left: track/task tree list
  - right: selected item details + live output
  - top/bottom: status and keybindings hints
- Keyboard-first controls, including pause/resume/quit and navigation.

### R2: Maestro-Native Semantics

- “Tasks” → “Tracks”
- Track expands to show plan tasks (tree structure)
- Status indicators: active/pending/completed, plus blocked/actionable if dependencies exist
- Setup experience integrated as first-run option in the Orchestrate pane (not a separate CLI-only flow)

### R3: Loop Mechanics

Implement Ralph-style loops with Maestro concepts:

- Two loop modes:
  - Planning: generate/update plan artifacts only (no implementation)
  - Building: implement tasks, run validation, commit, update plan
- Deterministic per-iteration state load and persistence (crash safe).
- Configurable error strategy (retry/skip/abort).
- Logs per iteration.

### R4: LeIndex Integration

Each iteration uses LeIndex to:

- locate relevant files/symbols
- produce balanced/ultra context bundles
- keep tokens minimal while staying actionable

## Acceptance Criteria

- Orchestrate pane can run an autonomous loop against a selected track with:
  - visible progress per task
  - persisted state
  - deterministic resume
- Track/task tree is rendered with expand/collapse and status indicators.
- Credits are added to `README.md`.

