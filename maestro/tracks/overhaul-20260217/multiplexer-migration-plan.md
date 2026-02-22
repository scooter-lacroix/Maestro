# Maestro Multiplexer Migration Plan: tmux → tab-rs

**Status:** Draft
**Created:** 2026-02-22
**Goal:** Migrate from tmux (via tmux-rs) to a forked tab-rs port ("maestro-tab") for unified cockpit integration

## Executive Summary

This plan details the migration from the current tmux-based session management to a custom fork of [tab-rs](https://github.com/austinjones/tab-rs), enabling:
- Native Rust terminal multiplexing without external tmux dependency
- Better transparency support (solving current OSC 111 issues)
- Future integration of conductor, memory, text editor, and maesterclaw into a unified multiplexer

---

## Part 1: Current Implementation Analysis

### 1.1 Files to Modify/Replace

| File | Lines | Purpose | Action |
|------|-------|---------|--------|
| `maestro/leindex/rust/src/multiplexer/tmux.rs` | 1227 | Core multiplexer implementation | **REPLACE** with `maestro_tab.rs` |
| `maestro/leindex/rust/src/multiplexer/mod.rs` | 6 | Module exports | **MODIFY** exports |
| `maestro/leindex/rust/src/multiplexer/zellij.rs` | 237 | Alternative multiplexer | **KEEP** as fallback |

### 1.2 Current tmux.rs Public API (Must Preserve)

```rust
// Lines 14-25: Status enum
pub enum TmuxSessionStatus {
    Active,    // GREEN: Content changed within cooldown
    Waiting,   // YELLOW: Content stable, user hasn't acknowledged
    Idle,      // GRAY: Content stable, user has acknowledged
    Error,     // Session doesn't exist or error
}

// Lines 28-34: Terminal capabilities
pub struct TerminalInfo {
    pub name: String,
    pub supports_osc8: bool,   // OSC 8 hyperlinks
    pub supports_osc52: bool,  // OSC 52 clipboard
    pub supports_true_color: bool,
}

// Lines 36-56: State tracking
pub struct StateTracker {
    pub last_hash: String,
    pub last_change_time: Instant,
    pub acknowledged: bool,
    pub acknowledged_at: Option<Instant>,
    pub last_activity_timestamp: i64,
}

// Lines 59-107: Session handle
pub struct TmuxSession {
    pub name: String,
    pub display_name: String,
    pub work_dir: String,
    pub command: Option<String>,
    pub created: Instant,
    pub state_tracker: StateTracker,
}
// Methods: new(), with_name(), log_file()

// Lines 110-118: Multiplexer instance
pub struct TmuxMultiplexer {
    session_cache: DashMap<String, i64>,
    cache_time: std::sync::RwLock<Option<Instant>>,
    cache_ttl: Duration,
}

// Lines 126-1114: Core methods (MUST IMPLEMENT)
impl TmuxMultiplexer {
    fn new() -> Self;                              // Line 128
    fn configure_global_transparency();            // Line 141
    fn is_available() -> Result<()>;               // Line 225
    fn refresh_session_cache(&self) -> Result<()>; // Line 243
    fn session_exists_from_cache(&self, name: &str) -> Option<bool>; // Line 282
    fn session_activity_from_cache(&self, name: &str) -> Option<i64>; // Line 294
    fn register_session_in_cache(&self, name: &str); // Line 306
    fn session_exists(&self, name: &str) -> bool;  // Line 314
    fn start_session(&self, session: &mut TmuxSession, command: Option<&str>) -> Result<()>; // Line 330
    fn configure_session_options(&self, session_name: &str) -> Result<()>; // Line 474
    fn configure_status_bar(&self, session: &TmuxSession) -> Result<()>; // Line 704
    fn enable_pipe_pane(&self, session: &TmuxSession) -> Result<()>; // Line 773
    fn send_keys(&self, session_name: &str, keys: &str) -> Result<()>; // Line 793
    fn send_enter(&self, session_name: &str) -> Result<()>; // Line 802
    fn attach(session_name: &str) -> Result<()>;   // Line 811
    fn get_environment(session_name: &str, var: &str) -> Result<Option<String>>; // Line 832
    fn set_environment(session_name: &str, var: &str, value: &str) -> Result<()>; // Line 857
    fn respawn_pane(session_name: &str, script: &str) -> Result<()>; // Line 875
    fn kill_session(&self, name: &str) -> Result<()>; // Line 892
    fn rename_session(&self, old_name: &str, new_display_name: &str) -> Result<String>; // Line 902
    fn fork_session(&self, original_name: &str, new_display_name: &str) -> Result<String>; // Line 926
    fn get_pane_content(session_name: &str, lines: usize) -> Result<String>; // Line 961
    fn get_window_activity(&self, session_name: &str) -> Option<i64>; // Line 979
    fn detect_terminal() -> TerminalInfo;          // Line 1001
    fn list_maestro_sessions(&self) -> Vec<String>; // Line 1060
    fn get_all_pane_paths(&self) -> Result<Vec<String>>; // Line 1071
    fn get_active_pane_path(&self) -> Result<Option<String>>; // Line 1097
}

// Lines 1116-1190: Helper functions
fn sanitize_name(name: &str) -> String;           // Line 1117
fn generate_short_id() -> String;                 // Line 1156
fn shell_quote(s: &str) -> String;                // Line 1168
```

### 1.3 Cockpit Integration Points

Files that import `TmuxMultiplexer`:

| File | Lines | Usage |
|------|-------|-------|
| `crates/cockpit/src/maestro_paths.rs` | 167, 177 | Path detection via `get_all_pane_paths()` |
| `crates/cockpit/src/conductor/pane.rs` | 19, 705 | Session creation and management |
| `crates/cockpit/src/conductor/observer.rs` | 328, 337, 346, 481 | Activity monitoring |
| `crates/cockpit/src/app.rs` | 32, 772, 1506, 1632, 4201, 4701, 4764, 4811, 4865, 4990 | Main TUI integration |

---

## Part 2: tab-rs Architecture Analysis

### 2.1 Crate Structure

```
tab-rs/
├── tab/                  # Main binary entry point
├── tab-command/          # CLI interface (6 files)
│   ├── lib.rs            # Command parsing
│   ├── config.rs         # Configuration
│   ├── bus.rs            # Message bus
│   ├── message/          # Message types
│   ├── service/          # Command services
│   └── state/            # State management
├── tab-daemon/           # Background daemon process (11 files)
│   ├── lib.rs            # Daemon entry, WebSocket server
│   ├── auth.rs           # Token authentication
│   ├── daemonfile.rs     # PID/state file
│   ├── bus/              # Internal messaging
│   ├── service/          # Daemon services
│   └── state/            # Session state
├── tab-pty/              # PTY process (7 files)
│   ├── lib.rs            # PTY entry, connects to daemon
│   ├── bus.rs            # PTY bus
│   ├── service/          # PTY services
│   └── message/          # PTY messages
├── tab-pty-process/      # Low-level PTY handling
└── common/               # Shared code
```

### 2.2 Key Architecture Patterns

1. **Daemon Process**: Persistent background process managing all tabs
   - Binds to `127.0.0.1:0` (random port)
   - WebSocket-based communication
   - Token authentication (128 byte)

2. **PTY Process**: Separate process for terminal I/O
   - Connects to daemon via WebSocket
   - Manages actual shell processes

3. **Command Process**: CLI interface
   - Communicates with daemon
   - Handles fuzzy finder, tab switching

4. **Lifeline Framework**: Message bus pattern
   ```rust
   // tab-daemon/src/lib.rs pattern
   let bus = DaemonBus::default();
   bus.store_resource::<DaemonConfig>(config);
   bus.store_resource::<WebsocketAuthToken>(auth_token.into());
   bus.store_resource::<WebsocketListenerResource>(websocket);
   ```

### 2.3 Performance Characteristics (from README)

- Tab launch: ~50ms
- Reconnect: ~10ms
- Keyboard latency: <5ms

---

## Part 3: Migration Implementation Plan

### Phase 1: Fork and Setup (Est. 2-3 days)

#### 1.1 Create maestro-tab Fork

```bash
# Fork location
mkdir -p maestro/leindex/rust/maestro-tab
cd maestro/leindex/rust/maestro-tab

# Clone and initialize
git clone https://github.com/austinjones/tab-rs.git .
git remote rename origin upstream
git remote add origin git@github.com:scooter-lmaestro/maestro-tab.git
```

#### 1.2 Files to Create

```
maestro/leindex/rust/maestro-tab/
├── Cargo.toml              # Update with maestro-tab naming
├── maestro-integration/    # NEW: Maestro-specific integration layer
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # MaestroTabMultiplexer implementation
│       ├── session.rs      # Session management (maps to tab API)
│       ├── transparency.rs # OSC 111 transparency handling
│       └── pty.rs          # PTY control extensions
└── [existing tab-rs files]
```

### Phase 2: API Compatibility Layer (Est. 3-4 days)

#### 2.1 Create Compatibility Shims

File: `maestro/leindex/rust/src/multiplexer/maestro_tab.rs`

```rust
//! Maestro Tab Multiplexer - Compatibility layer over tab-rs
//!
//! Provides TmuxMultiplexer-compatible API using tab-rs backend

use anyhow::{bail, Context, Result};
use dashmap::DashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

// Re-export types that remain unchanged
pub use super::tmux::{TmuxSessionStatus, TerminalInfo, StateTracker};

const SESSION_PREFIX: &str = "maestro_";

/// Session handle - compatible with TmuxSession
#[derive(Debug, Clone)]
pub struct MaestroTabSession {
    pub name: String,
    pub display_name: String,
    pub work_dir: String,
    pub command: Option<String>,
    pub created: Instant,
    pub state_tracker: StateTracker,
    // Tab-rs specific
    tab_id: Option<String>,
}

/// Maestro Tab Multiplexer - TmuxMultiplexer API over tab-rs
#[derive(Debug)]
pub struct MaestroTabMultiplexer {
    session_cache: DashMap<String, i64>,
    cache_time: std::sync::RwLock<Option<Instant>>,
    cache_ttl: Duration,
    daemon_client: Option<tab_daemon::Client>, // NEW: tab-rs client
}

// Implement all methods from TmuxMultiplexer...
```

#### 2.2 Method Mapping Table

| TmuxMultiplexer Method | tab-rs Equivalent | Implementation Notes |
|------------------------|-------------------|---------------------|
| `new()` | `tab_daemon::Client::connect()` | Connect to or spawn daemon |
| `start_session()` | `tab_api::tab::create()` | Create tab with working directory |
| `attach()` | `tab_pty::attach()` | Attach to PTY |
| `kill_session()` | `tab_api::tab::close()` | Close tab |
| `send_keys()` | Direct PTY write | Via WebSocket to PTY |
| `get_pane_content()` | `tab_api::buffer::capture()` | Capture terminal buffer |
| `list_maestro_sessions()` | `tab_api::tab::list()` | Filter by prefix |
| `session_exists()` | `tab_api::tab::get()` | Check tab exists |

### Phase 3: Transparency Fix (Est. 1-2 days)

#### 3.1 Problem Analysis

Current tmux transparency issue: OSC sequences sent via `send-keys` go to shell INPUT, not OUTPUT.

**Solution in tab-rs**: Direct PTY access allows writing escape sequences directly to terminal output.

#### 3.2 Implementation

File: `maestro/leindex/rust/maestro-tab/maestro-integration/src/transparency.rs`

```rust
//! Terminal Transparency Support
//!
//! Direct PTY output control for OSC sequences

use anyhow::Result;

/// Transparency reset sequence (OSC 111)
const TRANSPARENCY_SEQUENCE: &[u8] = b"\x1b[0m\x1b]111\x07\x1b[49m\x1b[2J\x1b[H";

pub struct TransparencyController {
    pty_writer: Box<dyn PtyWriter>,
}

impl TransparencyController {
    /// Apply transparency immediately after session creation
    pub fn apply_transparency(&self) -> Result<()> {
        // Direct write to PTY output - bypasses shell input
        self.pty_writer.write_output(TRANSPARENCY_SEQUENCE)?;
        Ok(())
    }

    /// Install shell hooks for persistent transparency
    pub fn install_shell_hooks(&self, shell: &str) -> Result<()> {
        match shell {
            "fish" => self.install_fish_hooks(),
            "bash" => self.install_bash_hooks(),
            "zsh" => self.install_zsh_hooks(),
            _ => Ok(()),
        }
    }
}
```

### Phase 4: Cockpit Integration Updates (Est. 2-3 days)

#### 4.1 Update Module Exports

File: `maestro/leindex/rust/src/multiplexer/mod.rs`

```rust
pub mod maestro_tab;  // NEW primary
pub mod tmux;         // Legacy fallback
pub mod zellij;       // Alternative fallback

// Use maestro_tab as primary
pub use maestro_tab::{
    MaestroTabMultiplexer,
    MaestroTabSession,
    MaestroTabSessionStatus,
    TerminalInfo,
};

// Type aliases for backwards compatibility
pub type TmuxMultiplexer = MaestroTabMultiplexer;
pub type TmuxSession = MaestroTabSession;
pub type TmuxSessionStatus = MaestroTabSessionStatus;
```

#### 4.2 Update Cockpit Imports

Files to update (no code changes, just verify compatibility):

- `crates/cockpit/src/maestro_paths.rs:167-177`
- `crates/cockpit/src/conductor/pane.rs:19,705`
- `crates/cockpit/src/conductor/observer.rs:328-481`
- `crates/cockpit/src/app.rs:32,772,1506,1632,4201,4701,4764,4811,4865,4990`

### Phase 5: Testing (Est. 2-3 days)

#### 5.1 Unit Tests

File: `maestro/leindex/rust/src/multiplexer/maestro_tab.rs` (tests module)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_name() {
        // Port from tmux.rs:1197-1203
        assert_eq!(sanitize_name("my-project"), "my-project");
        assert_eq!(sanitize_name("My Project!"), "My-Project");
    }

    #[test]
    fn test_session_creation() {
        // Verify session naming convention maintained
        let session = MaestroTabSession::new("Test", "/tmp");
        assert!(session.name.starts_with("maestro_Test_"));
    }

    #[test]
    fn test_transparency_sequence() {
        // Verify OSC 111 sequence is correct
        let seq = TransparencyController::get_sequence();
        assert!(seq.starts_with(b"\x1b[0m\x1b]111"));
    }
}
```

#### 5.2 Integration Tests

```bash
# Run full test suite
cargo test -p leindex-core -- multiplexer

# Test transparency
cargo test -p leindex-core -- transparency
```

---

## Part 4: Detailed File Changes

### 4.1 New Files to Create

| File | Lines (Est.) | Purpose |
|------|--------------|---------|
| `maestro/leindex/rust/maestro-tab/` | - | Fork of tab-rs |
| `maestro/leindex/rust/src/multiplexer/maestro_tab.rs` | ~800 | Main compatibility layer |
| `maestro/leindex/rust/src/multiplexer/transparency.rs` | ~150 | Transparency handling |
| `maestro/leindex/rust/src/multiplexer/pty_ext.rs` | ~200 | PTY extensions |

### 4.2 Files to Modify

| File | Changes |
|------|---------|
| `maestro/leindex/rust/src/multiplexer/mod.rs` | Add maestro_tab module, update exports |
| `maestro/leindex/rust/Cargo.toml` | Add maestro-tab dependency |
| `Cargo.toml` (workspace) | Add maestro-tab to workspace members |
| `crates/cockpit/Cargo.toml` | Verify leindex-core dep pulls new multiplexer |

### 4.3 Files to Deprecate (Keep as Fallback)

| File | Status |
|------|--------|
| `maestro/leindex/rust/src/multiplexer/tmux.rs` | Deprecated, keep for fallback |
| `maestro/leindex/rust/tmux-rs/` | Can be removed after migration verified |

---

## Part 5: Risk Assessment and Mitigation

### 5.1 Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| tab-rs daemon crashes | Medium | High | Fallback to tmux mode |
| WebSocket connection issues | Low | Medium | Retry logic, local socket fallback |
| PTY signal handling differences | Medium | Medium | Comprehensive signal tests |
| Performance regression | Low | Medium | Benchmark comparison |

### 5.2 Rollback Plan

```rust
// mod.rs with feature flag support
#[cfg(feature = "use-maestro-tab")]
pub use maestro_tab::{MaestroTabMultiplexer as TmuxMultiplexer, ...};

#[cfg(not(feature = "use-maestro-tab"))]
pub use tmux::{TmuxMultiplexer, ...};
```

---

## Part 6: Timeline

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| Phase 1: Fork & Setup | 2-3 days | None |
| Phase 2: API Compatibility | 3-4 days | Phase 1 |
| Phase 3: Transparency Fix | 1-2 days | Phase 2 |
| Phase 4: Cockpit Integration | 2-3 days | Phase 2, 3 |
| Phase 5: Testing | 2-3 days | Phase 4 |
| **Total** | **10-15 days** | |

---

## Part 7: Success Criteria

1. **Functional Parity**: All current TmuxMultiplexer methods work identically
2. **Transparency Works**: OSC 111 transparency applied on session creation
3. **Performance**: Session creation <100ms, keyboard latency <10ms
4. **No Breaking Changes**: Cockpit compiles and runs without modification
5. **Test Coverage**: All existing tests pass, new tests for transparency

---

## Appendix A: tab-rs Dependencies

From `tab-rs/Cargo.toml`:
- `tokio` - Async runtime
- `lifeline` - Message bus framework
- `anyhow` - Error handling
- `log`, `simplelog` - Logging
- `postage` - Async channels
- `tab-websocket` - WebSocket layer

## Appendix B: Key tab-rs Source References

| Component | File | Key Functions |
|-----------|------|---------------|
| Daemon entry | `tab-daemon/src/lib.rs:22-67` | `daemon_main()`, `new_bus()` |
| PTY entry | `tab-pty/src/lib.rs:16-58` | `pty_main()`, `spawn()` |
| Auth | `tab-daemon/src/auth.rs` | Token generation |
| Config | `tab-command/src/config.rs` | Configuration handling |
