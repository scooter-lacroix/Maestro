# API Compatibility Layer Design

**Track:** restructure-tab-migration_20260222
**Phase:** 2 - MaestroTab Integration
**Created:** 2026-02-22
**Status:** Design

---

## Overview

This document defines the API compatibility layer that enables `MaestroTabMultiplexer` (built on tab-rs) to implement the `TmuxMultiplexer` API. This allows existing Maestro code to work with the new tab-based multiplexer without code changes.

---

## 1. Type Aliases for Backward Compatibility

### 1.1 Primary Type Mappings

```rust
// src/leindex/src/multiplexer/mod.rs

// Type aliases for backward compatibility
pub use tab_multiplexer::MaestroTabMultiplexer as TmuxMultiplexer;
pub use tab_multiplexer::MaestroTabSession as TmuxSession;
pub use tab_multiplexer::TabStatus as TmuxSessionStatus;
pub use tab_multiplexer::TerminalInfo;

// Re-export at the module level
pub use {
    TmuxMultiplexer,
    TmuxSession,
    TmuxSessionStatus,
    TerminalInfo,
};
```

### 1.2 Feature Flag Control

```rust
// Cargo.toml for leindex-core

[features]
default = ["tmux-fallback"]
tmux = []                    # Use tmux directly
tab-rs = ["maestro-tab"]     # Use tab-rs daemon
tmux-fallback = ["tmux"]     # Fall back to tmux if daemon unavailable
maestro-tab = []             # Internal: link to maestro-tab crate
```

---

## 2. Method Mapping Table

### 2.1 Constructor and Connection

| TmuxMultiplexer Method | tab-rs Equivalent | Implementation Notes |
|------------------------|-------------------|----------------------|
| `new()` | `DaemonClient::connect()` | Connect to running daemon or spawn new one |
| `is_available()` | `DaemonClient::ping()` | Check daemon socket + fallback to tmux |
| N/A | `ensure_daemon()` | Start daemon if not running (uses tab-daemon) |

### 2.2 Session Lifecycle

| TmuxMultiplexer Method | tab-rs Equivalent | Implementation Notes |
|------------------------|-------------------|----------------------|
| `start_session(session, command)` | `Request::CreateTab(metadata)` | Send create request, wait for Init response |
| `session_exists(name)` | Check `InitResponse.tabs` | Query via daemon state or cached tab list |
| `kill_session(name)` | `Request::CloseTab(tab_id)` | Map name → TabId, send close request |
| `fork_session(old, new)` | Clone + CreateTab | Not native: create new tab with same metadata |

### 2.3 I/O Operations

| TmuxMultiplexer Method | tab-rs Equivalent | Implementation Notes |
|------------------------|-------------------|----------------------|
| `send_keys(name, keys)` | `Request::Input(tab_id, chunk)` | Direct PTY write via WebSocket |
| `send_enter(name)` | `InputChunk::from(b"\n")` | Convenience wrapper for send_keys |
| `get_pane_content(name, n)` | Scrollback buffer + live output | Subscribe to tab, capture recent chunks |

### 2.4 Attachment and Interaction

| TmuxMultiplexer Method | tab-rs Equivalent | Implementation Notes |
|------------------------|-------------------|----------------------|
| `attach(name)` | PTY接管 | Not applicable - Maestro uses websocket protocol, not直接接管终端 |
| `respawn_pane(name, script)` | Close + CreateTab | Kill existing, create new with command |
| `get_window_activity(name)` | `TabMetadata.selected` | Use tab's last-selected timestamp |

### 2.5 Environment and Configuration

| TmuxMultiplexer Method | tab-rs Equivalent | Implementation Notes |
|------------------------|-------------------|----------------------|
| `get_environment(name, var)` | `TabMetadata.env` | Read from metadata (cached at creation) |
| `set_environment(name, var, val)` | N/A (set at creation) | Must close and recreate with new env |
| `rename_session(old, new_name)` | Close + CreateTab | Tab IDs are immutable; need to recreate |

### 2.6 Query and Discovery

| TmuxMultiplexer Method | tab-rs Equivalent | Implementation Notes |
|------------------------|-------------------|----------------------|
| `list_maestro_sessions()` | Filter `InitResponse.tabs` | Filter tabs by `name.starts_with("maestro_")` |
| `get_all_pane_paths()` | Map `TabMetadata.dir` | Collect all tab directories |
| `get_active_pane_path()` | `TabMetadata.dir` (selected) | Get most recently selected tab's dir |
| `detect_terminal()` | `TerminalInfo::from_env()` | Query environment variables |

---

## 3. Error Handling Strategy

### 3.1 Fallback Hierarchy

```rust
pub enum MultiplexerBackend {
    TabRs(tab_client::DaemonClient),
    TmuxFallback(native_tmux::TmuxMultiplexer),
    Unavailable,
}

impl MultiplexerBackend {
    pub async fn connect() -> Self {
        // 1. Try to connect to tab-daemon
        if let Ok(client) = DaemonClient::connect().await {
            return Self::TabRs(client);
        }

        // 2. Try to spawn daemon
        if let Ok(client) = Self::spawn_and_connect().await {
            return Self::TabRs(client);
        }

        // 3. Fall back to tmux if feature enabled
        #[cfg(feature = "tmux-fallback")]
        if native_tmux::TmuxMultiplexer::is_available().is_ok() {
            return Self::TmuxFallback(native_tmux::TmuxMultiplexer::new());
        }

        Self::Unavailable
    }
}
```

### 3.2 Graceful Degradation

```rust
impl TmuxMultiplexer for MultiplexerBackend {
    fn start_session(&self, session: &mut TmuxSession, cmd: Option<&str>) -> Result<()> {
        match self {
            Self::TabRs(client) => {
                client.create_tab(session, cmd).await
                    .map_err(|e| anyhow!("Tab-rs failed: {}", e))
            }
            Self::TmuxFallback(mux) => {
                mux.start_session(session, cmd)
            }
            Self::Unavailable => {
                bail!("No multiplexer backend available")
            }
        }
    }
}
```

---

## 4. Feature Flag Design

### 4.1 Compilation Flags

```toml
[features]
default = ["tmux-fallback"]

# Prefer tab-rs, fall back to tmux
tmux-fallback = ["tmux"]

# Use only tab-rs (fail if unavailable)
tab-only = ["maestro-tab"]

# Use only tmux (legacy)
tmux = []

# Internal: link to local tab-rs fork
maestro-tab = []
```

### 4.2 Runtime Selection

```rust
pub struct MultiplexerConfig {
    /// Preferred backend (from env var or config)
    pub backend_preference: BackendPreference,

    /// Whether auto-fallback is enabled
    pub allow_fallback: bool,

    /// Daemon socket path
    pub socket_path: Option<PathBuf>,
}

pub enum BackendPreference {
    Auto,       // Try tab-rs, then tmux
    TabRs,      // Only tab-rs
    Tmux,       // Only tmux
}
```

### 4.3 Environment Variable Control

```bash
# Force specific backend
export MAESTRO_MULTIPLEXER=tab-rs
export MAESTRO_MULTIPLEXER=tmux

# Disable fallback
export MAESTRO_MULTIPLEXER_NO_FALLBACK=1

# Custom daemon socket
export MAESTRO_TAB_SOCKET=/tmp/maestro-tab.sock
```

---

## 5. Session Name Mapping

### 5.1 Tmux → Tab-rs Translation

```rust
pub struct SessionMapper {
    /// Prefix for Maestro sessions
    prefix: String,

    /// Name → TabId cache
    name_to_id: HashMap<String, TabId>,

    /// TabId → Metadata cache
    id_to_meta: HashMap<TabId, TabMetadata>,
}

impl SessionMapper {
    /// Tmux name → TabId lookup
    pub fn name_to_tabid(&self, name: &str) -> Option<TabId> {
        // Strip "maestro_" prefix
        let stripped = name.strip_prefix(self.prefix)?;
        self.name_to_id.get(stripped).copied()
    }

    /// TabId → Tmux-style name
    pub fn tabid_to_name(&self, id: TabId) -> Option<String> {
        let meta = self.id_to_meta.get(&id)?;
        Some(format!("{}{}", self.prefix, meta.name))
    }
}
```

### 5.2 Session Metadata Preservation

```rust
// TmuxSession stores original tmux-style name
pub struct TmuxSession {
    // Original tmux session name (for display)
    pub name: String,

    // Actual TabId (for tab-rs operations)
    pub tab_id: Option<TabId>,

    // Display name (user-visible)
    pub display_name: String,

    // Working directory
    pub work_dir: String,

    // State tracking (unchanged)
    pub state_tracker: StateTracker,
}
```

---

## 6. PTY Attachment Strategy

### 6.1 The Problem

- **tmux**: `tmux attach-session` takes over the terminal completely
- **tab-rs**: Uses WebSocket protocol, not raw PTY attachment

### 6.2 Maestro's Solution

Since Maestro Cockpit is a TUI that **observes** sessions rather than **attaching** to them:

1. **Content Capture**: Subscribe to tab output via WebSocket
2. **Input Injection**: Send keystrokes via WebSocket input
3. **No Raw PTY**: Never need raw PTY access in Cockpit

```rust
impl TabRsMultiplexer {
    /// "Attach" in Maestro means subscribe to output + enable input
    pub async fn attach_mode(&self, tab_id: TabId) -> Result<AttachedSession> {
        let (output_rx, input_tx) = self.subscribe(tab_id).await?;

        Ok(AttachedSession {
            tab_id,
            output: output_rx,
            input: input_tx,
        })
    }
}

pub struct AttachedSession {
    pub tab_id: TabId,
    pub output: tokio::sync::mpsc::Receiver<OutputChunk>,
    pub input: tokio::sync::mpsc::Sender<InputChunk>,
}
```

### 6.3 External Terminal Attachment

For users who want to open a session in an external terminal (not Cockpit):

```bash
# Use tab CLI to open in external terminal
tab select maestro_my-project

# This spawns a new terminal emulator connected to the tab
# via the tab-pty bridge
```

---

## 7. Scrollback Buffer Mapping

### 7.1 Tmux capture-pane → Tab-rs Buffer

```rust
impl TabRsMultiplexer {
    /// Get recent content (like tmux capture-pane)
    pub async fn get_pane_content(&self, tab_id: TabId, lines: usize) -> Result<String> {
        // 1. Subscribe to tab (triggers scrollback dump)
        let mut chunks = vec![];

        // 2. Receive initial scrollback buffer
        while let Some(chunk) = self.recv().await? {
            match chunk {
                Chunk::Scrollback(data) => {
                    chunks.extend(data.lines());
                    if chunks.len() >= lines {
                        break;
                    }
                }
                Chunk::Live(_) => {
                    // We've reached live data, scrollback exhausted
                    break;
                }
            }
        }

        // 3. Return last N lines
        let start = chunks.len().saturating_sub(lines);
        Ok(chunks[start..].join("\n"))
    }
}
```

---

## 8. Status Detection Mapping

### 8.1 TmuxSessionStatus → TabStatus

```rust
// tmux status enum (existing)
pub enum TmuxSessionStatus {
    Active,    // Content changed recently
    Waiting,   // Stable, unacknowledged
    Idle,      // Stable, acknowledged
    Error,     // Session error
}

// Map from tab-rs state
impl From<TabState> for TmuxSessionStatus {
    fn from(state: TabState) -> Self {
        match state {
            TabState::Running(has_output) => {
                if has_output {
                    TmuxSessionStatus::Active
                } else {
                    TmuxSessionStatus::Waiting
                }
            }
            TabState::Idle => TmuxSessionStatus::Idle,
            TabState::Terminated => TmuxSessionStatus::Error,
        }
    }
}
```

### 8.2 Activity Timestamp Mapping

```rust
// Tmux: window_activity (Unix timestamp)
// Tab-rs: TabMetadata.selected (Unix timestamp)

impl TabRsMultiplexer {
    pub fn get_window_activity(&self, tab_id: TabId) -> Option<i64> {
        self.metadata.get(&tab_id)
            .map(|m| m.selected as i64)
    }
}
```

---

## 9. Implementation Phases

### Phase 2.1: Core Bridge (1-2 days)

- [ ] Create `src/leindex/src/multiplexer/tab.rs` module
- [ ] Implement `MaestroTabMultiplexer` struct
- [ ] Implement `new()`, `is_available()`, `ensure_daemon()`
- [ ] Add feature flags to `Cargo.toml`

### Phase 2.2: Session Operations (2-3 days)

- [ ] Implement `start_session()` via CreateTab request
- [ ] Implement `session_exists()` via InitResponse
- [ ] Implement `kill_session()` via CloseTab request
- [ ] Add session name → TabId mapping cache

### Phase 2.3: I/O Bridge (2-3 days)

- [ ] Implement `send_keys()` via Input request
- [ ] Implement `get_pane_content()` via scrollback buffer
- [ ] Implement WebSocket subscription for live output
- [ ] Add output chunk aggregation

### Phase 2.4: Fallback Layer (1-2 days)

- [ ] Create `MultiplexerBackend` enum
- [ ] Implement fallback logic
- [ ] Add environment variable support
- [ ] Test fallback scenarios

### Phase 2.5: Testing & Validation (2-3 days)

- [ ] Unit tests for each method
- [ ] Integration tests with Cockpit
- [ ] Performance benchmarks (tmux vs tab-rs)
- [ ] Rollback test (feature flag toggle)

---

## 10. Open Questions

1. **PTY attach for debugging**: Should we provide a way to "attach" to a tab's PTY for debugging (similar to `docker exec`)?
   - *Proposal*: Add a `tab-pty` CLI command that uses the PTY bridge

2. **Session persistence**: tmux sessions survive daemon restarts; tab-rs tabs don't currently
   - *Proposal*: Implement session serialization to disk (future enhancement)

3. **Terminal capabilities**: tmux has extensive terminal feature detection; tab-rs relies on the PTY
   - *Decision*: Use `TerminalInfo::detect_terminal()` for both backends

4. **Performance**: WebSocket overhead vs tmux socket
   - *Mitigation*: Use TCP sockets locally, batch small chunks

---

## 11. Success Criteria

- [ ] All existing `TmuxMultiplexer` users work without code changes
- [ ] Cockpit displays tab-rs sessions correctly
- [ ] Fallback to tmux works when daemon unavailable
- [ ] Feature flag allows instant rollback
- [ ] Performance is within 2x of native tmux for common operations
- [ ] All tests pass with both `tab-only` and `tmux` features

---

## Appendix A: File Structure

```
src/leindex/src/multiplexer/
├── mod.rs              # Re-exports, feature gate
├── tmux.rs             # Existing tmux implementation (preserved)
├── tab.rs              # NEW: MaestroTabMultiplexer implementation
├── zellij.rs           # Existing zellij fallback (preserved)
└── backend.rs          # NEW: MultiplexerBackend enum
```

---

## Appendix B: Key Dependencies

```toml
[dependencies]
# Existing
anyhow = "1.0"
dashmap = "5.5"
tokio = { version = "1.0", features = ["full"] }

# New for tab-rs integration
maestro-tab = { path = "../../../maestro-tab/common/tab-api", optional = true }
tab-websocket = { path = "../../../maestro-tab/common/tab-websocket", optional = true }
```
