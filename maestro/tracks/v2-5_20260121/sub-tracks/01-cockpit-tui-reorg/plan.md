# Sub-Track 01: Cockpit v2 TUI Re-Org & Distribution - Plan

## Phase 1: Architecture & Layout

### [x] Task 1.1: Decide crate boundaries (Cockpit vs LeIndex core vs CLI)
- [x] Inventory current Rust modules under `maestro/leindex/rust/src/` and classify as: UI / core / adapters
- [x] Draft crate map:
  - `leindex-core`: analyzers + indexing + token formatting + 5-phase analysis
  - `maestro-cockpit`: ratatui UI + UI state + UI actions
  - `maestro-cli`: clap routing + subcommand glue (thin)
  - (optional) `maestro-orchestrate`: orchestration engine (library)
- [x] Define allowed dependency directions (one-way):
  - cockpit → core
  - cli → cockpit/core
  - core must not depend on cockpit

**Completion Note:** ADRs created at `docs/adr/001-cli-ownership-and-binary-naming.md` and `docs/adr/002-crate-reorganization.md`. Crate structure defined with workspace setup, dependency rules, and migration plan. Binary naming: `maestro-rs` (CLI), `maestro-setup` (wizard), `maestro-lsp-mcp-bridge` (LSP bridge).

### [x] Task 1.2: Define canonical paths (e.g., `crates/maestro-cockpit/`)
- [x] Choose and document final paths for all Rust crates (prefer `crates/`)
- [x] Decide what remains under `maestro/` (Python package + plugin assets) vs Rust workspace
- [x] Decide where archive/reference code lives and how it is prevented from runtime use

**Completion Note:** Canonical paths documented in ADR 002. Structure: `crates/cli/`, `crates/cockpit/`, `crates/lsp-bridge/`, `leindex/rust/` (core). Archive remains at `maestro/archive/` with CI gate to prevent runtime imports.

### [x] Task 1.3: Define binary naming (avoid Python/Rust `maestro` split-brain)
- [x] Pick a single installed end-user binary name (`maestro`) and define how it is produced
- [x] Decide how Python packaging behaves:
  - option A: no `console_scripts` for Python package; Rust installs `maestro`
  - option B: Python `maestro` becomes a thin delegator to Rust `maestro`
- [x] Update docs to reflect the decision (no ambiguous "two maestros")

**Completion Note:** Binary naming strategy defined in ADR 001 (Revised: Rust-only). The `maestro` binary is the sole CLI (all Rust). Legacy `maestro/cli.py` will be archived to `maestro/archive/legacy-python-cli/`. No Python CLI - all functionality in native Rust.

## Phase 2: Move + Modularize Cockpit

### [~] Task 2.1: Extract Cockpit UI code into the Cockpit crate
- [x] Move ratatui UI rendering + event loop into `maestro-cockpit` (app.rs created)
- [x] Move Cockpit state structs/enums into `maestro-cockpit` (keep core types in `leindex-core`)
- [ ] Ensure `maestro tui` still builds and runs after extraction

**Progress:** Workspace created, crates scaffolded, TUI code moved to `crates/cockpit/src/app.rs` (6000+ lines). Pending: Fix remaining compilation errors from import path changes (`leindex_analyzers` → `leindex_core`) and type mismatches.

### [ ] Task 2.2: Refactor monolithic UI into modules
- [ ] Establish module layout (example):
  - `cockpit/state/*` (state structs)
  - `cockpit/actions/*` (state transitions)
  - `cockpit/ui/*` (render functions)
  - `cockpit/tabs/*` (Dashboard/Sessions/Projects/Analysis/LSP/Settings/Orchestrate)
- [ ] Add a strict “no 5k-line file” constraint for new code (enforced by review)
- [ ] Replace ad-hoc cross-tab state mutations with explicit action handlers

### [ ] Task 2.3: Preserve behavior parity
- [ ] Sessions tab: list, create, fork, rename, kill, move-to-group
- [ ] MCP pool: start/stop/remove/sync, log viewer
- [ ] LSP: status cache + toggle/restart + log viewer + install guidance
- [ ] Memory: list/search, project browser integration
- [ ] Settings: theme/editor/install path configuration
- [ ] Analysis: keep functional until Sub-Track 02 rewires it to LeIndex phases
  - (temporary) keep existing analysis hub commands working

## Phase 3: Wire `maestro tui`

### [ ] Task 3.1: Ensure CLI routes to Cockpit crate
- [ ] CLI `Tui` subcommand imports Cockpit crate and calls `cockpit::run()`
- [ ] Verify `maestro tui` exits cleanly and restores terminal on panic/error paths

### [ ] Task 3.2: Ensure config loading remains stable (`~/.maestro/config.toml`)
- [ ] Keep config schema stable or provide a migration path
- [ ] Ensure config path resolution works with `MAESTRO_PROFILE` and other env vars

### [ ] Task 3.3: Ensure tmux multiplexer behavior remains stable
- [ ] Validate tmux target resolution logic in `Implement` path
- [ ] Validate tmux session naming constraints and escaping
- [ ] Validate behavior when tmux is absent (graceful degradation)

## Phase 4: Retire Go TUI Wiring

### [ ] Task 4.1: Update `Makefile` targets (remove Go TUI, add Rust targets)
- [ ] Replace `tui-build/tui-install/tui-test` with Rust equivalents (`cargo build`, `cargo test`)
- [ ] Ensure `make install-all` produces a working `maestro` binary

### [ ] Task 4.2: Update `/maestro:tui` command docs to reflect Rust Cockpit
- [ ] Update `claude-code/commands/maestro:tui.md` instructions (no Go, no `maestro/tui`)
- [ ] Ensure docs match actual install paths and binary names

### [ ] Task 4.3: Update `plugin.json` CLI dependency list (remove Go requirement)
- [ ] Remove Go dependency from `plugin.json`
- [ ] Ensure `plugin.json.commands` matches command files present in `claude-code/commands`

### [ ] Task 4.4: Confirm no runtime path references `maestro/archive/tui-go`
- [ ] Ripgrep gate: forbid `archive/tui-go` references in runtime code paths
- [ ] Keep `maestro/archive/tui-go` as reference only

## Phase 5: Installer / Build Pipeline

### [ ] Task 5.1: Update `install.sh` and/or wizard to install Rust binaries to `~/.local/bin`
- [ ] Install: build release binaries and copy/symlink into `~/.local/bin`
- [ ] Verify: `maestro tui` works immediately after install
- [ ] Ensure uninstallation/upgrade story is documented (avoid orphaned binaries)

### [ ] Task 5.2: Add CI build check for Cockpit binary
- [ ] Add CI job: `cargo build --release` (or equivalent workspace build)
- [ ] Add CI job: minimal smoke test that `maestro tui` starts (headless flag or short-run mode)

### [ ] Task 5.3: Document build instructions in `README.md`
- [ ] Add `cargo build` and `cargo install --path ...` instructions
- [ ] Remove old Go-specific instructions

## Phase 6: Verification

### [ ] Task 6.1: Manual verification on Linux (local)
- [ ] Launch + navigate all existing tabs
- [ ] Validate tmux flows (create session, send implement command)

### [ ] Task 6.2: Manual verification on macOS/WSL (documented commands)
- [ ] Validate config resolution paths
- [ ] Validate terminal rendering and keybindings

### [ ] Task 6.3: Maestro - User Manual Verification 'Sub-Track 01' (Protocol in workflow.md)
