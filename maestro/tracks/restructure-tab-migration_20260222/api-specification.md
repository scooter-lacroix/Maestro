# MaestroTabMultiplexer API Specification

**Track:** restructure-tab-migration_20260222
**Phase:** 2 - Tab Multiplexer Migration
**Purpose:** Document the exact API contract that MaestroTabMultiplexer must implement

---

## Overview

The MaestroTabMultiplexer must be a drop-in replacement for TmuxMultiplexer. All public types and methods must have identical signatures to maintain backward compatibility with Cockpit.

---

## Public Types

### Enum: TmuxSessionStatus (or MaestroTabSessionStatus)

```rust
/// Session status for the 3-state model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TmuxSessionStatus {
    /// GREEN: Content changed within cooldown period
    Active,
    /// YELLOW: Content stable, user hasn't acknowledged
    Waiting,
    /// GRAY: Content stable, user has acknowledged
    Idle,
    /// Session doesn't exist or error
    Error,
}
```

### Struct: TerminalInfo

```rust
/// Terminal capabilities detected from environment
#[derive(Debug, Clone)]
pub struct TerminalInfo {
    pub name: String,
    pub supports_osc8: bool,  // OSC 8 hyperlinks
    pub supports_osc52: bool, // OSC 52 clipboard
    pub supports_true_color: bool,
}
```

### Struct: StateTracker

```rust
/// State tracker for notification-style status detection
#[derive(Debug, Clone)]
pub struct StateTracker {
    pub last_hash: String,
    pub last_change_time: Instant,
    pub acknowledged: bool,
    pub acknowledged_at: Option<Instant>,
    pub last_activity_timestamp: i64,
}
```

### Struct: TmuxSession (or MaestroTabSession)

```rust
/// A session handle
#[derive(Debug, Clone)]
pub struct TmuxSession {
    pub name: String,
    pub display_name: String,
    pub work_dir: String,
    pub command: Option<String>,
    pub created: Instant,
    pub state_tracker: StateTracker,
}
```

**Methods:**
- `pub fn new(display_name: &str, work_dir: &str) -> Self`
- `pub fn with_name(name: String, display_name: &str, work_dir: &str) -> Self`
- `pub fn log_file(&self) -> std::path::PathBuf`

### Struct: TmuxMultiplexer (or MaestroTabMultiplexer)

```rust
/// Multiplexer - manages terminal sessions
#[derive(Debug)]
pub struct TmuxMultiplexer {
    session_cache: DashMap<String, i64>,
    cache_time: std::sync::RwLock<Option<Instant>>,
    cache_ttl: Duration,
}
```

---

## Public Methods

### Constructor

```rust
pub fn new() -> Self
```
Creates a new multiplexer instance.

### Availability Check

```rust
pub fn is_available() -> Result<()>
```
Check if the multiplexer backend is available.

### Session Cache Methods

```rust
pub fn refresh_session_cache(&self) -> Result<()>
pub fn session_exists_from_cache(&self, name: &str) -> Option<bool>
pub fn session_activity_from_cache(&self, name: &str) -> Option<i64>
pub fn register_session_in_cache(&self, name: &str)
pub fn session_exists(&self, name: &str) -> bool
```

### Session Lifecycle Methods

```rust
pub fn start_session(&self, session: &mut TmuxSession, command: Option<&str>) -> Result<()>
pub fn kill_session(&self, name: &str) -> Result<()>
pub fn rename_session(&self, old_name: &str, new_display_name: &str) -> Result<String>
pub fn fork_session(&self, original_name: &str, new_display_name: &str) -> Result<String>
```

### Session Input/Output

```rust
pub fn send_keys(&self, session_name: &str, keys: &str) -> Result<()>
pub fn send_enter(&self, session_name: &str) -> Result<()>
pub fn get_pane_content(session_name: &str, lines: usize) -> Result<String>
```

### Static Attachment

```rust
pub fn attach(session_name: &str) -> Result<()>
```
**Note:** This is a static method (no `&self`).

### Environment Management

```rust
pub fn get_environment(session_name: &str, var: &str) -> Result<Option<String>>
pub fn set_environment(session_name: &str, var: &str, value: &str) -> Result<()>
```

### Pane/Window Management

```rust
pub fn respawn_pane(session_name: &str, script: &str) -> Result<()>
pub fn get_window_activity(&self, session_name: &str) -> Option<i64>
pub fn get_all_pane_paths(&self) -> Result<Vec<String>>
pub fn get_active_pane_path(&self) -> Result<Option<String>>
```

### Session Listing

```rust
pub fn list_maestro_sessions(&self) -> Vec<String>
```

### Terminal Detection

```rust
pub fn detect_terminal() -> TerminalInfo
```
**Note:** This is a static method (no `&self`).

---

## Helper Functions (Private, but must be replicated)

```rust
fn sanitize_name(name: &str) -> String
fn generate_short_id() -> String
fn shell_quote(s: &str) -> String
```

---

## Module Exports (mod.rs)

```rust
pub mod maestro_tab;
pub mod tmux;     // Legacy fallback
pub mod zellij;   // Alternative fallback

// Type aliases for backward compatibility
pub type TmuxMultiplexer = maestro_tab::MaestroTabMultiplexer;
pub type TmuxSession = maestro_tab::MaestroTabSession;
pub type TmuxSessionStatus = maestro_tab::MaestroTabSessionStatus;

// Re-exports
pub use maestro_tab::{TerminalInfo, StateTracker};
```

---

## Integration Points in Cockpit

The following files import TmuxMultiplexer and must work without modification:

1. `crates/cockpit/src/app.rs:32`
2. `crates/cockpit/src/maestro_paths.rs:167,177`
3. `crates/cockpit/src/conductor/pane.rs:19,705`
4. `crates/cockpit/src/conductor/observer.rs:328-481`

---

## Feature Flag Design (for rollback)

```toml
# In Cargo.toml
[features]
default = ["maestro-tab"]
maestro-tab = []
tmux-fallback = []
```

```rust
// In mod.rs
#[cfg(feature = "maestro-tab")]
pub type TmuxMultiplexer = maestro_tab::MaestroTabMultiplexer;

#[cfg(feature = "tmux-fallback")]
pub type TmuxMultiplexer = tmux::TmuxMultiplexer;
```

---

## Performance Requirements

| Metric | Target | Notes |
|--------|--------|-------|
| Session creation | <100ms | Target: 50ms |
| Keyboard latency | <10ms | Target: <5ms |
| Reconnection | <50ms | Target: 10ms |

---

## Transparency Support

The multiplexer must support OSC 111 transparency sequences for foot terminal:
- Apply transparency on session creation
- Support shell hooks for persistent transparency
- Direct PTY output access (solves current tmux issue)

---

## Error Handling

All methods return `anyhow::Result<T>`. The multiplexer should:
1. Return errors for truly exceptional conditions
2. Use the `Error` status for non-existent sessions
3. Implement graceful fallback to tmux if daemon unavailable

---

## Testing Requirements

1. Unit tests for helper functions (sanitize_name, shell_quote, etc.)
2. Integration tests for session lifecycle
3. Transparency tests on foot terminal
4. Performance benchmarks
