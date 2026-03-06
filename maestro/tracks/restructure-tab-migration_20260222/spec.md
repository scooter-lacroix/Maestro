# Specification: Maestro Restructure & Tab Multiplexer Migration

**Track ID:** restructure-tab-migration_20260222
**Type:** Feature / Refactoring
**Status:** New
**Created:** 2026-02-22
**Estimated Duration:** 15-20 days

---

## Overview

This is a comprehensive two-phase track that fundamentally reorganizes the Maestro project structure and migrates the terminal multiplexer from tmux to a forked tab-rs implementation.

**Phase 1: Project Restructuring**
- Surface LeIndex core and CLI from the deeply nested `maestro/leindex/rust/` folder
- Unify into a proper package structure with centralized `src/` organization
- The `maestro/` folder should ONLY contain templates for AI tools and maestro tracks
- All functionality must be preserved during restructuring

**Phase 2: Tab Multiplexer Migration**
- Complete migration from tmux (tmux-rs) to a forked tab-rs port ("maestro-tab")
- Implement using the detailed plan in `multiplexer-migration-plan.md`
- Ensure no existing functionality is lost
- Migration must be seamless with robust implementation

---

## Phase 1: Project Restructuring

### 1.1 Current State

**Problematic Structure:**
```
maestro/
├── leindex/rust/           # Should NOT be nested under maestro/
│   ├── src/
│   ├── Cargo.toml
│   └── tmux-rs/
├── tracks/                 # CORRECT: Track definitions
├── claude-code/            # CORRECT: AI tool templates
└── opencode/               # CORRECT: AI tool templates
```

**Target Structure:**
```
/
├── src/                    # NEW: Unified source organization
│   ├── leindex/            # LeIndex core (from maestro/leindex/rust/src/)
│   ├── cli/                # CLI entry point
│   ├── multiplexer/        # Terminal multiplexers
│   ├── orchestrate/        # Conductor engine
│   ├── memory/             # Memory service
│   └── ...
├── crates/                 # Existing: Cockpit, pi-mono, etc.
├── maestro/                # ONLY templates and tracks
│   ├── tracks/
│   ├── claude-code/
│   └── opencode/
├── Cargo.toml              # Workspace root
└── [existing root files]
```

### 1.2 Functional Requirements

#### FR-1.1: LeIndex Core Relocation
- Move `maestro/leindex/rust/src/` contents to `src/leindex/`
- Update all module paths from `leindex_core::` to appropriate new paths
- Preserve all existing imports and exports
- Zero breaking changes to public APIs

#### FR-1.2: CLI Surface Preservation
- Relocate CLI entry point to `src/cli/` or `crates/cli/`
- Preserve all CLI commands: `tui`, `memory`, `analyze`, `le-index`
- Maintain all subcommands and argument parsing
- Shell completion scripts must continue to work

#### FR-1.3: Multiplexer Module Organization
- Consolidate multiplexer implementations under `src/multiplexer/`
- Preserve `tmux.rs`, `zellij.rs`, and new `maestro_tab.rs`
- Maintain module exports and type aliases for backward compatibility

#### FR-1.4: Orchestrate Module
- Keep orchestrate engine at `src/orchestrate/`
- Preserve all models, parser, engine, and agent runner integrations
- Maintain Cockpit integration points

#### FR-1.5: Memory Service
- Preserve memory service at `src/memory/`
- Maintain MCP pool, LSP pool, and session management
- Keep Turso storage backend intact

### 1.3 Non-Functional Requirements

#### NFR-1.1: Zero Functionality Loss
- All existing tests must pass after restructuring
- No changes to public API contracts
- Documentation must remain accurate

#### NFR-1.2: Build System
- Update `Cargo.toml` workspace members
- Update `Makefile` build targets
- Preserve all build commands (`make build`, `make test`, `make lint`)

#### NFR-1.3: Path Resolution
- Update all import paths across codebase
- Update documentation references
- Update LeIndex path configuration

### 1.4 Files to Move/Modify

**Files to Relocate:**
```
maestro/leindex/rust/src/          → src/leindex/
maestro/leindex/rust/Cargo.toml    → src/leindex/Cargo.toml (workspace member)
maestro/leindex/rust/tmux-rs/      → [remove after Phase 2 migration]
```

**Files to Modify (Import Path Updates):**
- `crates/cockpit/src/*.rs` (10+ files importing `leindex_core::`)
- `crates/cli/src/main.rs`
- `maestro/leindex/rust/Cargo.toml` → workspace `Cargo.toml`
- Root `Cargo.toml`

**Documentation to Update:**
- `README.md`
- `CLAUDE.md`
- `docs/` directory contents
- `maestro/*.md`

### 1.5 Acceptance Criteria (Phase 1)

1. **AC-1.1:** All source files relocated to `src/` hierarchy
2. **AC-1.2:** `maestro/` folder contains only tracks and AI tool templates
3. **AC-1.3:** All 193+ tests pass after restructuring
4. **AC-1.4:** `cargo build --workspace` succeeds
5. **AC-1.5:** `make test` passes completely
6. **AC-1.6:** CLI commands work: `maestro tui`, `maestro memory`, `maestro analyze`
7. **AC-1.7:** LeIndex commands work: `maestro le-index search`, `maestro le-index analyze`
8. **AC-1.8:** TUI launches and all tabs function correctly
9. **AC-1.9:** No clippy warnings introduced
10. **AC-1.10:** All documentation updated with new paths

---

## Phase 2: Tab Multiplexer Migration

### 2.1 Current Implementation

**tmux-based multiplexer:**
- Located at `maestro/leindex/rust/src/multiplexer/tmux.rs` (1227 lines)
- Implements `TmuxMultiplexer` with 25+ methods
- Manages sessions, panes, transparency, status bar
- Integration points in 18+ cockpit files

**Known Issues:**
- Transparency (OSC 111) not working on session launch
- Requires external tmux binary
- Complex escape sequence handling

### 2.2 Target Implementation

**tab-rs fork ("maestro-tab"):**
- Native Rust terminal multiplexer
- WebSocket-based daemon architecture
- Direct PTY output access (solves transparency)
- Performance: 50ms launch, 10ms reconnect, <5ms latency

**Architecture:**
```
maestro-tab/
├── tab-daemon/           # Background daemon (WebSocket server)
├── tab-pty/              # PTY process
├── tab-command/          # CLI interface
├── maestro-integration/  # Maestro-specific layer
│   ├── src/
│   │   ├── lib.rs        # MaestroTabMultiplexer
│   │   ├── session.rs    # Session management
│   │   ├── transparency.rs
│   │   └── pty.rs
└── common/               # Shared code
```

### 2.3 Functional Requirements

#### FR-2.1: API Compatibility
- Implement all 25+ `TmuxMultiplexer` methods
- Preserve `TmuxSession`, `TmuxSessionStatus`, `TerminalInfo` types
- Maintain backward compatibility via type aliases

#### FR-2.2: Session Management
- `start_session()` - Create new session with working directory
- `attach()` - Attach to existing session
- `kill_session()` - Terminate session
- `list_maestro_sessions()` - List all maestro sessions
- `session_exists()` - Check session existence

#### FR-2.3: Content and State
- `get_pane_content()` - Capture terminal buffer (last N lines)
- `send_keys()` - Send input to session
- `get_window_activity()` - Get activity timestamp
- `get_all_pane_paths()` - Get all pane working directories

#### FR-2.4: Environment and Configuration
- `get_environment()` / `set_environment()` - Environment variables
- `configure_session_options()` - Mouse, clipboard, history
- `configure_status_bar()` - Status bar with session info

#### FR-2.5: Transparency Support
- Direct PTY output for OSC 111 sequences
- Apply transparency on session creation
- Shell hooks for persistent transparency
- Support foot terminal with alpha transparency

### 2.4 Method Mapping

| TmuxMultiplexer Method | tab-rs Equivalent | Notes |
|------------------------|-------------------|-------|
| `new()` | Daemon client connect | Spawn or connect to daemon |
| `start_session()` | Tab creation API | Set working directory |
| `attach()` | PTY attach | WebSocket to PTY |
| `send_keys()` | Direct PTY write | Via daemon message |
| `get_pane_content()` | Buffer capture | Terminal buffer API |
| `kill_session()` | Tab close | Cleanup resources |
| `list_maestro_sessions()` | Tab list | Filter by prefix |
| `session_exists()` | Tab get | Existence check |

### 2.5 Cockpit Integration

**Files requiring NO code changes (via type aliases):**
- `crates/cockpit/src/maestro_paths.rs:167-177`
- `crates/cockpit/src/conductor/pane.rs:19,705`
- `crates/cockpit/src/conductor/observer.rs:328-481`
- `crates/cockpit/src/app.rs:32,772,1506,1632,4201,4701,4764,4811,4865,4990`

**Module exports update:**
```rust
// src/multiplexer/mod.rs
pub mod maestro_tab;  // NEW primary
pub mod tmux;         // Legacy fallback
pub mod zellij;       // Alternative fallback

pub use maestro_tab::{
    MaestroTabMultiplexer as TmuxMultiplexer,
    MaestroTabSession as TmuxSession,
    MaestroTabSessionStatus as TmuxSessionStatus,
    TerminalInfo, StateTracker,
};
```

### 2.6 Non-Functional Requirements

#### NFR-2.1: Performance
- Session creation: <100ms (target: 50ms)
- Keyboard latency: <10ms (target: <5ms)
- No performance regression vs tmux

#### NFR-2.2: Reliability
- Daemon auto-restart on crash
- WebSocket reconnection with exponential backoff
- Graceful fallback to tmux if daemon unavailable

#### NFR-2.3: Security
- Token authentication (128-byte random)
- Localhost-only binding (127.0.0.1)
- No Origin header in WebSocket requests

### 2.7 Dependencies

**New dependencies from tab-rs:**
- `tokio` - Async runtime
- `lifeline` - Message bus framework
- `anyhow` - Error handling
- `postage` - Async channels
- `tab-websocket` - WebSocket layer

**Version pinning:**
- All new dependencies must use compatible versions
- Avoid version conflicts with existing crates

### 2.8 Acceptance Criteria (Phase 2)

1. **AC-2.1:** All 25+ TmuxMultiplexer methods implemented
2. **AC-2.2:** Cockpit compiles without modification
3. **AC-2.3:** All existing tests pass
4. **AC-2.4:** New unit tests for maestro-tab (sanitization, session creation, transparency)
5. **AC-2.5:** Integration tests (session lifecycle, content capture)
6. **AC-2.6:** Transparency works on foot terminal (OSC 111 applied)
7. **AC-2.7:** Performance benchmarks: session <100ms, latency <10ms
8. **AC-2.8:** Daemon auto-start and reconnection
9. **AC-2.9:** Fallback to tmux on daemon failure
10. **AC-2.10:** Code coverage >98% for new multiplexer code

---

## Out of Scope

1. **Rewriting Cockpit UI** - Only integration points, no UI changes
2. **Zellij integration** - Keep as fallback, no enhancements
3. **Multi-architecture support** - Linux only for initial release
4. **Windows/macOS support** - Future consideration
5. **Remote session support** - Local only initially

---

## Dependencies

### Phase 1 Dependencies
- None (can start immediately)

### Phase 2 Dependencies
- Phase 1 must be complete (multiplexer at new location)
- tab-rs fork must be created
- External dependencies verified available

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Tests passing | 100% (193+) | `cargo test --workspace` |
| Build time | <2 min | `cargo build --release` |
| Session creation | <100ms | Benchmark |
| Transparency working | Yes | Visual verification |
| Code coverage | >98% | `cargo-tarpaulin` |
| Zero clippy warnings | 0 | `cargo clippy` |

---

## Rollback Plan

### Phase 1 Rollback
- Git revert to pre-restructure commit
- All changes atomic (single commit per file group)

### Phase 2 Rollback
- Feature flag: `use-maestro-tab`
- Revert to tmux by disabling feature
- Keep tmux.rs as fallback

---

## Documentation Requirements

1. **Updated CLAUDE.md** with new project structure
2. **Architecture Decision Record** for restructuring
3. **Tab Multiplexer Integration Guide**
4. **Migration Guide** for contributors
5. **API Documentation** (rustdoc)
