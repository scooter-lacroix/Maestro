# Implementation Plan: Maestro Restructure & Tab Multiplexer Migration

**Track ID:** restructure-tab-migration_20260222
**Created:** 2026-02-22

---

## Phase 1: Project Restructuring

**Goal:** Surface LeIndex core and CLI from `maestro/leindex/rust/` to unified `src/` structure. The `maestro/` folder should ONLY contain templates and tracks.

### Task 1.1: Analysis and Planning

- [x] Task 1.1.1: Map all files in `maestro/leindex/rust/src/` to new locations
- [x] Task 1.1.2: Identify all import paths requiring updates (grep `leindex_core::`)
- [x] Task 1.1.3: Document workspace dependency graph
- [x] Task 1.1.4: Create backup branch for rollback safety

### Task 1.2: Create New Directory Structure

- [x] Task 1.2.1: Create `src/` directory at repository root
- [x] Task 1.2.2: Create `src/leindex/` for LeIndex core
- [x] Task 1.2.3: Create `src/cli/` for CLI entry point (if separate from leindex)
- [x] Task 1.2.4: Create `src/multiplexer/` for terminal multiplexers
- [x] Task 1.2.5: Create `src/orchestrate/` for conductor engine
- [x] Task 1.2.6: Create `src/memory/` for memory service

### Task 1.3: Relocate LeIndex Core

- [x] Task 1.3.1: Move `maestro/leindex/rust/src/lib.rs` → `src/leindex/lib.rs`
- [x] Task 1.3.2: Move `maestro/leindex/rust/src/memory/` → `src/leindex/memory/`
- [x] Task 1.3.3: Move `maestro/leindex/rust/src/orchestrate/` → `src/leindex/orchestrate/`
- [x] Task 1.3.4: Move `maestro/leindex/rust/src/multiplexer/` → `src/leindex/multiplexer/`
- [x] Task 1.3.5: Move `maestro/leindex/rust/src/lsp/` → `src/leindex/lsp/`
- [x] Task 1.3.6: Move `maestro/leindex/rust/src/analyze/` → `src/leindex/analyze/`
- [x] Task 1.3.7: Verify all files moved correctly

### Task 1.4: Update LeIndex Cargo.toml

- [x] Task 1.4.1: Create `src/leindex/Cargo.toml` from `maestro/leindex/rust/Cargo.toml`
- [x] Task 1.4.2: Update `[package]` section with new paths
- [x] Task 1.4.3: Update `[dependencies]` for relative path changes
- [x] Task 1.4.4: Update `[dev-dependencies]` if applicable

### Task 1.5: Update Root Workspace Configuration

- [x] Task 1.5.1: Update root `Cargo.toml` workspace members
- [x] Task 1.5.2: Add `src/leindex` to workspace members
- [x] Task 1.5.3: Add `src/cli` (if separate) to workspace members
- [x] Task 1.5.4: Remove `maestro/leindex/rust` from workspace members
- [x] Task 1.5.5: Verify `cargo build --workspace` succeeds

### Task 1.6: Update Import Paths in Cockpit

- [x] Task 1.6.1: Update `crates/cockpit/src/app.rs:32` (`use leindex_core::multiplexer::TmuxMultiplexer`)
- [x] Task 1.6.2: Update all other imports in `crates/cockpit/src/app.rs`
- [x] Task 1.6.3: Update `crates/cockpit/src/maestro_paths.rs:167,177`
- [x] Task 1.6.4: Update `crates/cockpit/src/conductor/pane.rs:19,705`
- [x] Task 1.6.5: Update `crates/cockpit/src/conductor/observer.rs:328-481`
- [x] Task 1.6.6: Update `crates/cockpit/src/conductor/tests.rs`
- [x] Task 1.6.7: Update all remaining cockpit imports
- [x] Task 1.6.8: Verify cockpit compiles

### Task 1.7: Update Import Paths in CLI

- [x] Task 1.7.1: Update `crates/cli/src/main.rs` imports
- [x] Task 1.7.2: Update CLI subcommand modules
- [x] Task 1.7.3: Verify `maestro` binary commands work

### Task 1.8: Update Import Paths in Other Crates

- [x] Task 1.8.1: Update `crates/pi-mono/` imports
- [x] Task 1.8.2: Update `crates/lsp-bridge/` imports
- [x] Task 1.8.3: Update any other crate imports
- [x] Task 1.8.4: Run `cargo build --workspace` to verify all

### Task 1.9: Update Makefile Build Targets

- [x] Task 1.9.1: Update `make build` target for new paths
- [x] Task 1.9.2: Update `make test` target
- [x] Task 1.9.3: Update `make lint` target
- [x] Task 1.9.4: Update `make check` target
- [x] Task 1.9.5: Test all Makefile commands

### Task 1.10: Update Documentation

- [x] Task 1.10.1: Update `README.md` with new project structure
- [x] Task 1.10.2: Update `CLAUDE.md` with new paths
- [x] Task 1.10.3: Update `docs/` directory contents
- [x] Task 1.10.4: Update ADRs (Architecture Decision Records) if any
- [x] Task 1.10.5: Update skill definitions that reference paths

### Task 1.11: Clean Up maestro Directory

- [x] Task 1.11.1: Verify `maestro/` now only contains tracks and templates
- [x] Task 1.11.2: Remove empty `maestro/leindex/` directory
- [x] Task 1.11.3: Create `.gitkeep` in `maestro/tracks/` if needed
- [x] Task 1.11.4: Verify maestro templates intact

### Task 1.12: Testing and Verification

- [ ] Task 1.12.1: Run `cargo test --workspace` (all 193+ tests must pass)
- [ ] Task 1.12.2: Run `cargo clippy --workspace` (zero warnings)
- [ ] Task 1.12.3: Run `cargo fmt --all --check` (formatting)
- [ ] Task 1.12.4: Test `maestro tui` command
- [ ] Task 1.12.5: Test `maestro memory status` command
- [ ] Task 1.12.6: Test `maestro analyze` command
- [ ] Task 1.12.7: Test `maestro le-index search` command
- [ ] Task 1.12.8: Verify TUI all tabs work (Dashboard, Sessions, Projects, LSP, Memory, ktop, Orchestrate)
- [ ] Task 1.12.9: Run `make policy-check` (architectural rules)

### Task 1.13: Create Restructure ADR

- [ ] Task 1.13.1: Create `docs/adr/004-project-restructuring.md`
- [ ] Task 1.13.2: Document rationale for restructuring
- [ ] Task 1.13.3: Document before/after structure
- [ ] Task 1.13.4: Document migration path

### Task 1.14: Create Git Commit

- [ ] Task 1.14.1: Stage all restructuring changes
- [ ] Task 1.14.2: Create commit with message: `feat(restructure): Surface LeIndex to src/ hierarchy`
- [ ] Task 1.14.3: Verify commit hash

### Task 1.15: Phase Completion Verification

- [ ] Task 1.15.1: Final verification of all acceptance criteria
- [ ] Task 1.15.2: Update plan.md with commit hashes
- [ ] Task 1.15.3: Mark Phase 1 complete in plan.md

---

## Phase 2: Tab Multiplexer Migration

**Goal:** Complete migration from tmux to forked tab-rs ("maestro-tab") per `multiplexer-migration-plan.md`.

### Task 2.1: Fork and Setup tab-rs

- [ ] Task 2.1.1: Create `src/maestro-tab/` directory for forked tab-rs
- [ ] Task 2.1.2: Clone tab-rs from `https://github.com/austinjones/tab-rs.git`
- [ ] Task 2.1.3: Initialize git repository with proper remotes
- [ ] Task 2.1.4: Verify tab-rs builds standalone
- [ ] Task 2.1.5: Document tab-rs dependencies

### Task 2.2: Create Maestro Integration Layer

- [ ] Task 2.2.1: Create `src/maestro-tab/maestro-integration/Cargo.toml`
- [ ] Task 2.2.2: Create `src/maestro-tab/maestro-integration/src/lib.rs`
- [ ] Task 2.2.3: Create `src/maestro-tab/maestro-integration/src/session.rs`
- [ ] Task 2.2.4: Create `src/maestro-tab/maestro-integration/src/transparency.rs`
- [ ] Task 2.2.5: Create `src/maestro-tab/maestro-integration/src/pty.rs`

### Task 2.3: Implement MaestroTabMultiplexer Core

- [ ] Task 2.3.1: Define `MaestroTabMultiplexer` struct
- [ ] Task 2.3.2: Define `MaestroTabSession` struct (compatible with TmuxSession)
- [ ] Task 2.3.3: Implement `new()` constructor (daemon connection)
- [ ] Task 2.3.4: Implement session cache (DashMap)
- [ ] Task 2.3.5: Implement cache TTL and refresh logic

### Task 2.4: Implement Session Management Methods

- [ ] Task 2.4.1: Implement `session_exists()` - check tab exists
- [ ] Task 2.4.2: Implement `session_exists_from_cache()` - cached check
- [ ] Task 2.4.3: Implement `session_activity_from_cache()` - cached activity
- [ ] Task 2.4.4: Implement `register_session_in_cache()` - cache registration
- [ ] Task 2.4.5: Implement `refresh_session_cache()` - cache refresh

### Task 2.5: Implement Session Lifecycle Methods

- [ ] Task 2.5.1: Implement `start_session()` - create new tab
- [ ] Task 2.5.2: Implement `attach()` - attach to existing tab
- [ ] Task 2.5.3: Implement `kill_session()` - terminate tab
- [ ] Task 2.5.4: Implement `list_maestro_sessions()` - list all tabs
- [ ] Task 2.5.5: Implement `rename_session()` - rename tab
- [ ] Task 2.5.6: Implement `fork_session()` - duplicate tab

### Task 2.6: Implement Content and State Methods

- [ ] Task 2.6.1: Implement `get_pane_content()` - capture buffer
- [ ] Task 2.6.2: Implement `send_keys()` - send input to tab
- [ ] Task 2.6.3: Implement `send_enter()` - send Enter key
- [ ] Task 2.6.4: Implement `get_window_activity()` - get activity timestamp
- [ ] Task 2.6.5: Implement `get_all_pane_paths()` - get working directories
- [ ] Task 2.6.6: Implement `get_active_pane_path()` - get active path

### Task 2.7: Implement Environment and Configuration

- [ ] Task 2.7.1: Implement `get_environment()` - get env var
- [ ] Task 2.7.2: Implement `set_environment()` - set env var
- [ ] Task 2.7.3: Implement `configure_session_options()` - mouse, clipboard, etc.
- [ ] Task 2.7.4: Implement `configure_status_bar()` - status bar setup
- [ ] Task 2.7.5: Implement `enable_pipe_pane()` - logging setup

### Task 2.8: Implement Transparency Support

- [ ] Task 2.8.1: Define OSC 111 transparency sequence
- [ ] Task 2.8.2: Implement `apply_transparency()` - direct PTY output
- [ ] Task 2.8.3: Implement shell hooks (fish, bash, zsh)
- [ ] Task 2.8.4: Integrate transparency into `start_session()`
- [ ] Task 2.8.5: Test transparency on foot terminal

### Task 2.9: Implement PTY Extensions

- [ ] Task 2.9.1: Create PTY writer trait
- [ ] Task 2.9.2: Implement WebSocket to PTY bridge
- [ ] Task 2.9.3: Implement signal handling (SIGWINCH, etc.)
- [ ] Task 2.9.4: Implement error recovery and reconnect

### Task 2.10: Implement Helper Functions

- [ ] Task 2.10.1: Implement `sanitize_name()` - from tmux.rs
- [ ] Task 2.10.2: Implement `generate_short_id()` - unique IDs
- [ ] Task 2.10.3: Implement `shell_quote()` - shell escaping
- [ ] Task 2.10.4: Implement `detect_terminal()` - terminal detection

### Task 2.11: Create Unit Tests

- [ ] Task 2.11.1: Test `sanitize_name()` (port from tmux.rs)
- [ ] Task 2.11.2: Test `generate_short_id()` uniqueness
- [ ] Task 2.11.3: Test `shell_quote()` escaping
- [ ] Task 2.11.4: Test session creation (naming convention)
- [ ] Task 2.11.5: Test transparency sequence validity
- [ ] Task 2.11.6: Test cache TTL logic
- [ ] Task 2.11.7: Test terminal detection

### Task 2.12: Update Multiplexer Module

- [ ] Task 2.12.1: Move existing `src/leindex/multiplexer/tmux.rs` content to compatibility layer
- [ ] Task 2.12.2: Update `src/leindex/multiplexer/mod.rs` with maestro_tab exports
- [ ] Task 2.12.3: Add type aliases for backward compatibility
- [ ] Task 2.12.4: Add feature flag `use-maestro-tab` for rollback
- [ ] Task 2.12.5: Verify module compiles

### Task 2.13: Update Workspace Dependencies

- [ ] Task 2.13.1: Add `src/maestro-tab` to root `Cargo.toml` workspace
- [ ] Task 2.13.2: Add maestro-tab dependencies to workspace
- [ ] Task 2.13.3: Pin dependency versions (tokio, lifeline, etc.)
- [ ] Task 2.13.4: Verify `cargo build --workspace` succeeds

### Task 2.14: Integration Testing - Session Lifecycle

- [ ] Task 2.14.1: Test daemon auto-start
- [ ] Task 2.14.2: Test session creation
- [ ] Task 2.14.3: Test session attachment
- [ ] Task 2.14.4: Test session termination
- [ ] Task 2.14.5: Test session listing

### Task 2.15: Integration Testing - Content and State

- [ ] Task 2.15.1: Test pane content capture
- [ ] Task 2.15.2: Test send_keys functionality
- [ ] Task 2.15.3: Test activity tracking
- [ ] Task 2.15.4: Test working directory detection

### Task 2.16: Integration Testing - Cockpit

- [ ] Task 2.16.1: Test Cockpit compiles without changes
- [ ] Task 2.16.2: Test TUI session creation
- [ ] Task 2.16.3: Test TUI session attachment
- [ ] Task 2.16.4: Test conductor pane functionality
- [ ] Task 2.16.5: Test observer functionality

### Task 2.17: Performance Benchmarking

- [ ] Task 2.17.1: Benchmark session creation time (target <100ms)
- [ ] Task 2.17.2: Benchmark keyboard latency (target <10ms)
- [ ] Task 2.17.3: Benchmark cache refresh
- [ ] Task 2.17.4: Compare against tmux baseline
- [ ] Task 2.17.5: Document performance results

### Task 2.18: Transparency Testing

- [ ] Task 2.18.1: Test OSC 111 sequence on foot terminal
- [ ] Task 2.18.2: Verify transparency on session creation
- [ ] Task 2.18.3: Test shell hooks installation
- [ ] Task 2.18.4: Verify persistent transparency

### Task 2.19: Error Handling and Fallback

- [ ] Task 2.19.1: Implement daemon crash detection
- [ ] Task 2.19.2: Implement WebSocket reconnection logic
- [ ] Task 2.19.3: Implement fallback to tmux mode
- [ ] Task 2.19.4: Test error recovery scenarios
- [ ] Task 2.19.5: Test fallback activation

### Task 2.20: Documentation

- [ ] Task 2.20.1: Document maestro-tab architecture
- [ ] Task 2.20.2: Document API compatibility layer
- [ ] Task 2.20.3: Document transparency implementation
- [ ] Task 2.20.4: Update CLAUDE.md with multiplexer info
- [ ] Task 2.20.5: Create troubleshooting guide

### Task 2.21: Code Coverage Verification

- [ ] Task 2.21.1: Run `cargo-tarpaulin` on maestro-tab
- [ ] Task 2.21.2: Verify >98% coverage
- [ ] Task 2.21.3: Add tests for any uncovered code
- [ ] Task 2.21.4: Document coverage report

### Task 2.22: Final Testing and Verification

- [ ] Task 2.22.1: Run all 193+ existing tests (must all pass)
- [ ] Task 2.22.2: Run clippy (zero warnings)
- [ ] Task 2.22.3: Run formatting check
- [ ] Task 2.22.4: Test all CLI commands
- [ ] Task 2.22.5: Test TUI all tabs
- [ ] Task 2.22.6: Run `make policy-check`

### Task 2.23: Create Git Commits

- [ ] Task 2.23.1: Commit tab-rs fork
- [ ] Task 2.23.2: Commit maestro integration layer
- [ ] Task 2.23.3: Commit API compatibility implementation
- [ ] Task 2.23.4: Commit tests and documentation
- [ ] Task 2.23.5: Tag release: `v2.6.0-tab-multiplexer`

### Task 2.24: Phase Completion Verification

- [ ] Task 2.24.1: Verify all acceptance criteria met
- [ ] Task 2.24.2: Update plan.md with all commit hashes
- [ ] Task 2.24.3: Mark Phase 2 complete in plan.md

---

## Phase 3: Final Integration and Polish

### Task 3.1: Cross-Phase Integration Testing

- [ ] Task 3.1.1: Verify restructured paths work with tab multiplexer
- [ ] Task 3.1.2: Test full workflow: CLI → TUI → multiplexer → transparency
- [ ] Task 3.1.3: Test all entry points
- [ ] Task 3.1.4: Verify no regressions

### Task 3.2: Documentation Updates

- [ ] Task 3.2.1: Update main README.md
- [ ] Task 3.2.2: Update CLAUDE.md
- [ ] Task 3.2.3: Update architecture diagrams
- [ ] Task 3.2.4: Create migration guide for contributors
- [ ] Task 3.2.5: Update CHANGELOG.md

### Task 3.3: Final Verification

- [ ] Task 3.3.1: Run full test suite (all tests pass)
- [ ] Task 3.3.2: Run clippy (zero warnings)
- [ ] Task 3.3.3: Run formatting check
- [ ] Task 3.3.4: Verify build times acceptable
- [ ] Task 3.3.5: Verify performance targets met

### Task 3.4: Tzar Review Preparation

- [ ] Task 3.4.1: Prepare phase summary for Tzar review
- [ ] Task 3.4.2: Gather all commit hashes
- [ ] Task 3.4.3: Document all file changes
- [ ] Task 3.4.4: Prepare test results

### Task 3.5: Tzar Review

- [ ] Task 3.5.1: Invoke codex-reviewer with "Tzar of Excellence" directive
- [ ] Task 3.5.2: Address all critical findings
- [ ] Task 3.5.3: Re-test after fixes
- [ ] Task 3.5.4: Document review results
- [ ] Task 3.5.5: Obtain final verdict

### Task 3.6: Final Commits and Release

- [ ] Task 3.6.1: Commit any Tzar-mandated fixes
- [ ] Task 3.6.2: Create release tag: `v2.6.0`
- [ ] Task 3.6.3: Update tracks.md with completion status
- [ ] Task 3.6.4: Store track completion in memory

---

## Task Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Phase 1: Restructuring | 15 tasks | New |
| Phase 2: Tab Migration | 24 tasks | New |
| Phase 3: Integration | 6 tasks | New |
| **Total** | **45 tasks** | **New** |

---

## Dependencies

- Phase 2 depends on Phase 1 (multiplexer must be at new location)
- Phase 3 depends on Phase 2 (tab migration must be complete)

---

## Timeline Estimate

| Phase | Estimated Duration |
|-------|-------------------|
| Phase 1: Restructuring | 5-7 days |
| Phase 2: Tab Migration | 10-15 days |
| Phase 3: Integration | 2-3 days |
| **Total** | **17-25 days** |
