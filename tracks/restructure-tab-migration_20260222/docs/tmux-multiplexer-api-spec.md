# TmuxMultiplexer API Specification

> Generated for Phase 2: Maestro Tab Multiplexer Migration
> This document defines the complete public API contract that must be preserved in the new `maestro-tab` multiplexer compatibility layer.

## Overview

The `TmuxMultiplexer` is the primary terminal session manager in Maestro. It provides:
- Session lifecycle management (create, attach, kill, rename, fork)
- State tracking with a 3-state notification model (Active/Waiting/Idle)
- Caching for efficient session queries
- Terminal capability detection
- Environment variable management
- Pane content capture

**File Location:** `src/leindex/src/multiplexer/tmux.rs` (1,255 lines)

## Public API Exports

### Module Re-exports (`src/leindex/src/multiplexer/mod.rs`)

```rust
pub use tmux::{
    TerminalInfo,
    TmuxMultiplexer,
    TmuxSession,
    TmuxSessionStatus,
};
```

---

## Public Enums

### `TmuxSessionStatus`

**Location:** Lines 14-25

```rust
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

**Purpose:** 3-state model for notification-style status detection in the TUI.

---

## Public Structs

### `TerminalInfo`

**Location:** Lines 27-34

```rust
#[derive(Debug, Clone)]
pub struct TerminalInfo {
    pub name: String,
    pub supports_osc8: bool,      // OSC 8 hyperlinks
    pub supports_osc52: bool,     // OSC 52 clipboard
    pub supports_true_color: bool,
}
```

**Purpose:** Terminal capabilities detected from environment variables.

### `StateTracker`

**Location:** Lines 36-56

```rust
#[derive(Debug, Clone)]
pub struct StateTracker {
    pub last_hash: String,
    pub last_change_time: Instant,
    pub acknowledged: bool,
    pub acknowledged_at: Option<Instant>,
    pub last_activity_timestamp: i64,
}
```

**Purpose:** Tracks session state for notification-style status detection.

**Default Implementation:**
```rust
impl Default for StateTracker {
    fn default() -> Self {
        Self {
            last_hash: String::new(),
            last_change_time: Instant::now(),
            acknowledged: false,
            acknowledged_at: None,
            last_activity_timestamp: 0,
        }
    }
}
```

### `TmuxSession`

**Location:** Lines 58-107

```rust
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

**Purpose:** Handle representing a tmux session with metadata.

#### Methods

| Method | Signature | Purpose |
|--------|-----------|---------|
| `new` | `pub fn new(display_name: &str, work_dir: &str) -> Self` | Create new session with auto-generated unique name (`maestro_{sanitized}_{id}`) |
| `with_name` | `pub fn with_name(name: String, display_name: &str, work_dir: &str) -> Self` | Create session with specific name (for restoration) |
| `log_file` | `pub fn log_file(&self) -> std::path::PathBuf` | Get log file path: `~/.maestro/logs/{name}.log` |

### `TmuxMultiplexer`

**Location:** Lines 109-1142

```rust
#[derive(Debug)]
pub struct TmuxMultiplexer {
    session_cache: DashMap<String, i64>,      // name -> activity timestamp
    cache_time: std::sync::RwLock<Option<Instant>>,
    cache_ttl: Duration,                       // Default: 2 seconds
}
```

**Purpose:** Main multiplexer managing tmux sessions with caching.

#### Trait Implementations

```rust
impl Default for TmuxMultiplexer {
    fn default() -> Self {
        Self::new()  // Calls configure_global_transparency()
    }
}
```

---

## Public Methods - `TmuxMultiplexer`

### Constructor

| Method | Signature | Notes |
|--------|-----------|-------|
| `new` | `pub fn new() -> Self` | Calls `configure_global_transparency()` on first use |

### Availability Checks

| Method | Signature | Returns |
|--------|-----------|---------|
| `is_available` | `pub fn is_available() -> Result<()>` | `Ok(())` if tmux is installed and working |

### Session Cache Management

| Method | Signature | Purpose |
|--------|-----------|---------|
| `refresh_session_cache` | `pub fn refresh_session_cache(&self) -> Result<()>` | Refresh cache from `tmux list-sessions` (O(1) subprocess) |
| `session_exists_from_cache` | `pub fn session_exists_from_cache(&self, name: &str) -> Option<bool>` | Check existence from cache (returns `None` if cache stale) |
| `session_activity_from_cache` | `pub fn session_activity_from_cache(&self, name: &str) -> Option<i64>` | Get activity timestamp from cache (returns `None` if stale) |
| `register_session_in_cache` | `pub fn register_session_in_cache(&self, name: &str)` | Register newly created session (prevents race condition) |

### Session Lifecycle

| Method | Signature | Purpose |
|--------|-----------|---------|
| `session_exists` | `pub fn session_exists(&self, name: &str) -> bool` | Check if session exists (cache-aware, falls back to direct check) |
| `start_session` | `pub fn start_session(&self, session: &mut TmuxSession, command: Option<&str>) -> Result<()>` | Create new tmux session with transparency, logging, optional initial command |
| `attach` | `pub fn attach(session_name: &str) -> Result<()>` | Attach to session (static method) |
| `kill_session` | `pub fn kill_session(&self, name: &str) -> Result<()>` | Kill session and remove from cache |
| `rename_session` | `pub fn rename_session(&self, old_name: &str, new_display_name: &str) -> Result<String>` | Rename session, returns new internal name |
| `fork_session` | `pub fn fork_session(&self, original_name: &str, new_display_name: &str) -> Result<String>` | Create new session based on original, returns new name |

### Input/Output

| Method | Signature | Purpose |
|--------|-----------|---------|
| `send_keys` | `pub fn send_keys(&self, session_name: &str, keys: &str) -> Result<()>` | Send keystrokes to session |
| `send_enter` | `pub fn send_enter(&self, session_name: &str) -> Result<()>` | Send Enter key to session |
| `get_pane_content` | `pub fn get_pane_content(session_name: &str, lines: usize) -> Result<String>` | Capture last N lines of pane output |

### Environment Variables

| Method | Signature | Purpose |
|--------|-----------|---------|
| `get_environment` | `pub fn get_environment(session_name: &str, var: &str) -> Result<Option<String>>` | Get session environment variable (returns `Ok(None)` if unset/session missing) |
| `set_environment` | `pub fn set_environment(session_name: &str, var: &str, value: &str) -> Result<()>` | Set session environment variable |

### Pane Management

| Method | Signature | Purpose |
|--------|-----------|---------|
| `respawn_pane` | `pub fn respawn_pane(session_name: &str, script: &str) -> Result<()>` | Respawn pane 0.0 with new shell script using `sh -lc` |

### Activity and Path Queries

| Method | Signature | Purpose |
|--------|-----------|---------|
| `get_window_activity` | `pub fn get_window_activity(&self, session_name: &str) -> Option<i64>` | Get window activity timestamp (cache-aware) |
| `get_all_pane_paths` | `pub fn get_all_pane_paths(&self) -> Result<Vec<String>>` | Get all pane current paths (deduplicated, sorted) |
| `get_active_pane_path` | `pub fn get_active_pane_path(&self) -> Result<Option<String>>` | Get active pane's current path |

### Session Listing

| Method | Signature | Purpose |
|--------|-----------|---------|
| `list_maestro_sessions` | `pub fn list_maestro_sessions(&self) -> Vec<String>` | List all sessions starting with `maestro_` prefix |

### Terminal Detection

| Method | Signature | Returns |
|--------|-----------|---------|
| `detect_terminal` | `pub fn detect_terminal() -> TerminalInfo` | Detected terminal capabilities (static method) |

**Detected Terminals:**
- `warp`, `iterm2`, `kitty`, `alacritty`, `vscode`, `windows-terminal`, `wezterm`, `apple-terminal`, `unknown`

---

## Private Helper Functions (For Reference)

These functions are internal but may need equivalents in the new multiplexer:

| Function | Signature | Purpose |
|----------|-----------|---------|
| `sanitize_name` | `fn sanitize_name(name: &str) -> String` | Convert display name to valid tmux session name (alphanumeric + `-`, max 50 chars) |
| `generate_short_id` | `fn generate_short_id() -> String` | Generate 8-char hex ID from nanosecond timestamp |
| `shell_quote` | `fn shell_quote(s: &str) -> String` | Quote string for safe shell use (single-quote escaping) |

---

## Constants

```rust
const SESSION_PREFIX: &str = "maestro_";
```

---

## Behavior Notes

### Session Naming Convention
- Format: `maestro_{sanitized_display_name}_{short_id}`
- Example: `maestro_My-Project_a3f2b1c4`

### Transparency Configuration
The multiplexer configures extensive tmux settings for terminal transparency support:
- Global options (server-level): `window-style`, `window-active-style`, pane borders, terminal overrides
- Session options: remain-on-exit, passthrough, hyperlinks
- Environment variables: `TERMINAL_HAS_TRANSPARENCY=1`, `COLORTERM=truecolor`
- Shell hooks: Fish, Bash, Zsh hooks in `~/.maestro/` and `~/.config/fish/conf.d/`

### Cache Behavior
- TTL: 2 seconds (configurable via `cache_ttl` field)
- Refreshed via `refresh_session_cache()`
- Checked via `session_exists_from_cache()` and `session_activity_from_cache()`
- Race condition prevention via `register_session_in_cache()`

### Logging
- Location: `~/.maestro/logs/{session_name}.log`
- Method: `pipe-pane -o` with shell-quoting for safety

### Environment Variables Passed to New Sessions
- `HOME`, `PATH`, `CLAUDE_CONFIG_DIR`, `COLORTERM`, `TERM`, `MAESTRO_TRANSPARENCY=1`

---

## Test Coverage

Located at lines 1220-1254:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_sanitize_name()
    #[test]
    fn test_shell_quote()
    #[test]
    fn test_generate_short_id()
    #[test]
    fn test_tmux_session_new()
}
```

---

## Usage in Other Modules

The TmuxMultiplexer is used by:

1. **`crates/cockpit/src/conductor/pane.rs`** - Session management for conductor tracks
2. **`crates/cockpit/src/conductor/observer.rs`** - Activity monitoring
3. **`crates/cockpit/src/app.rs`** - Main TUI application
4. **`src/leindex/src/memory/service.rs`** - Memory service sessions
5. **`src/leindex/src/memory/session_manager.rs`** - Session lifecycle management
6. **`src/leindex/src/cli/implement.rs`** - CLI commands
7. **`crates/cockpit/src/maestro_paths.rs`** - Path utilities

---

## Compatibility Layer Requirements

When implementing `maestro-tab` multiplexer, the compatibility layer must:

1. **Preserve all public method signatures** - Drop-in replacement for `TmuxMultiplexer`
2. **Maintain session naming convention** - `maestro_` prefix for discoverability
3. **Implement the same caching strategy** - 2-second TTL with `DashMap<String, i64>`
4. **Support the 3-state status model** - `TmuxSessionStatus` (Active/Waiting/Idle/Error)
5. **Provide terminal detection** - `TerminalInfo` with OSC 8/52 and truecolor detection
6. **Handle transparency configuration** - Or equivalent for non-tmux backends
7. **Support environment variable management** - `get_environment`/`set_environment`
8. **Implement pane operations** - `send_keys`, `send_enter`, `get_pane_content`, `respawn_pane`
9. **Provide session listing** - `list_maestro_sessions` filtering by prefix
10. **Support session restoration** - `with_name` constructor for recovering sessions

---

## Future Considerations

### Abstract Multiplexer Trait
For Phase 2+, consider defining a `Multiplexer` trait:

```rust
#[async_trait]
pub trait Multiplexer: Send + Sync {
    // Session lifecycle
    fn start_session(&self, session: &mut dyn Session, command: Option<&str>) -> Result<()>;
    fn attach(&self, session_name: &str) -> Result<()>;
    fn kill_session(&self, name: &str) -> Result<()>;
    fn session_exists(&self, name: &str) -> bool;

    // I/O
    fn send_keys(&self, session_name: &str, keys: &str) -> Result<()>;
    fn get_pane_content(&self, session_name: &str, lines: usize) -> Result<String>;

    // Queries
    fn list_sessions(&self) -> Vec<String>;
    fn get_activity(&self, session_name: &str) -> Option<i64>;
}
```

### Backend Support
- **Tmux** - Current implementation
- **Zellij** - Partial support exists in `zellij.rs`
- **maestro-tab** - New native backend (this migration)
- **Detachless** - Future: Direct PTY management without multiplexer

---

*Generated: 2026-02-22*
*Source: `src/leindex/src/multiplexer/tmux.rs` (commit: feature/restructure-tab-migration-20260222)*
