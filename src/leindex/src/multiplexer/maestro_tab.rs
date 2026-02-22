//! MaestroTab Multiplexer Module
//!
//! Compatibility layer for tab management via tab-rs (forked).
//! Currently delegates to TmuxMultiplexer for incremental migration.
//!
//! ## Architecture
//!
//! This module provides a unified multiplexer API that will eventually use tab-rs
//! for native Rust session management. The current implementation delegates to the
//! existing TmuxMultiplexer to maintain functionality while tab-rs integration is
//! developed incrementally.
//!
//! ## Migration Path
//!
//! Phase 1 (Current): Delegate all operations to TmuxMultiplexer
//! Phase 2: Implement tab-rs binary interface via subprocess
//! Phase 3: Direct tab-rs library integration (pending version resolution)

use anyhow::Result;
use std::process::Command;
use tracing::debug;

use super::tmux::{StateTracker, TerminalInfo, TmuxMultiplexer, TmuxSession, TmuxSessionStatus};

/// MaestroTab session status (mirrors TmuxSessionStatus)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaestroTabSessionStatus {
    /// GREEN: Content changed within cooldown period
    Active,
    /// YELLOW: Content stable, user hasn't acknowledged
    Waiting,
    /// GRAY: Content stable, user has acknowledged
    Idle,
    /// Session doesn't exist or error
    Error,
}

impl From<TmuxSessionStatus> for MaestroTabSessionStatus {
    fn from(status: TmuxSessionStatus) -> Self {
        match status {
            TmuxSessionStatus::Active => MaestroTabSessionStatus::Active,
            TmuxSessionStatus::Waiting => MaestroTabSessionStatus::Waiting,
            TmuxSessionStatus::Idle => MaestroTabSessionStatus::Idle,
            TmuxSessionStatus::Error => MaestroTabSessionStatus::Error,
        }
    }
}

impl From<MaestroTabSessionStatus> for TmuxSessionStatus {
    fn from(status: MaestroTabSessionStatus) -> Self {
        match status {
            MaestroTabSessionStatus::Active => TmuxSessionStatus::Active,
            MaestroTabSessionStatus::Waiting => TmuxSessionStatus::Waiting,
            MaestroTabSessionStatus::Idle => TmuxSessionStatus::Idle,
            MaestroTabSessionStatus::Error => TmuxSessionStatus::Error,
        }
    }
}

/// MaestroTab session handle (mirrors TmuxSession)
#[derive(Debug, Clone)]
pub struct MaestroTabSession {
    pub name: String,
    pub display_name: String,
    pub work_dir: String,
    pub command: Option<String>,
    pub created: std::time::Instant,
    pub state_tracker: StateTracker,
}

impl From<TmuxSession> for MaestroTabSession {
    fn from(session: TmuxSession) -> Self {
        Self {
            name: session.name,
            display_name: session.display_name,
            work_dir: session.work_dir,
            command: session.command,
            created: session.created,
            state_tracker: session.state_tracker,
        }
    }
}

impl From<MaestroTabSession> for TmuxSession {
    fn from(session: MaestroTabSession) -> Self {
        Self {
            name: session.name,
            display_name: session.display_name,
            work_dir: session.work_dir,
            command: session.command,
            created: session.created,
            state_tracker: session.state_tracker,
        }
    }
}

impl MaestroTabSession {
    /// Create a new session handle (generates unique name)
    pub fn new(display_name: &str, work_dir: &str) -> Self {
        TmuxSession::new(display_name, work_dir).into()
    }

    /// Create session with a specific name (for restoration)
    pub fn with_name(name: String, display_name: &str, work_dir: &str) -> Self {
        TmuxSession::with_name(name, display_name, work_dir).into()
    }

    /// Get the log file path for this session
    pub fn log_file(&self) -> std::path::PathBuf {
        let tmux_session: TmuxSession = self.clone().into();
        tmux_session.log_file()
    }
}

/// MaestroTab multiplexer - manages terminal sessions
///
/// Currently delegates to TmuxMultiplexer for all operations.
/// Future versions will integrate tab-rs for native Rust session management.
#[derive(Debug)]
pub struct MaestroTabMultiplexer {
    /// Inner tmux multiplexer (fallback implementation)
    inner: TmuxMultiplexer,
}

impl Default for MaestroTabMultiplexer {
    fn default() -> Self {
        Self::new()
    }
}

impl MaestroTabMultiplexer {
    /// Create a new multiplexer instance
    pub fn new() -> Self {
        debug!("Initializing MaestroTabMultiplexer (delegating to tmux)");
        Self {
            inner: TmuxMultiplexer::new(),
        }
    }

    /// Check if the tab backend is available
    pub fn is_available() -> Result<()> {
        // Currently delegates to tmux
        TmuxMultiplexer::is_available()
    }

    /// Refresh the session cache (call once per tick)
    pub fn refresh_session_cache(&self) -> Result<()> {
        self.inner.refresh_session_cache()
    }

    /// Check if session exists from cache
    pub fn session_exists_from_cache(&self, name: &str) -> Option<bool> {
        self.inner.session_exists_from_cache(name)
    }

    /// Get session activity from cache
    pub fn session_activity_from_cache(&self, name: &str) -> Option<i64> {
        self.inner.session_activity_from_cache(name)
    }

    /// Register a newly created session in cache
    pub fn register_session_in_cache(&self, name: &str) {
        self.inner.register_session_in_cache(name)
    }

    /// Check if a session exists
    pub fn session_exists(&self, name: &str) -> bool {
        self.inner.session_exists(name)
    }

    /// Start a new session
    pub fn start_session(&self, session: &mut MaestroTabSession, command: Option<&str>) -> Result<()> {
        let mut tmux_session: TmuxSession = session.clone().into();
        self.inner.start_session(&mut tmux_session, command)?;
        // Update the original session with any changes made during start
        *session = tmux_session.into();
        Ok(())
    }

    /// Send keys to a session
    pub fn send_keys(&self, session_name: &str, keys: &str) -> Result<()> {
        self.inner.send_keys(session_name, keys)
    }

    /// Send Enter key to a session
    pub fn send_enter(&self, session_name: &str) -> Result<()> {
        self.inner.send_enter(session_name)
    }

    /// Attach to a session
    pub fn attach(session_name: &str) -> Result<()> {
        TmuxMultiplexer::attach(session_name)
    }

    /// Get an environment variable value for a session
    pub fn get_environment(session_name: &str, var: &str) -> Result<Option<String>> {
        TmuxMultiplexer::get_environment(session_name, var)
    }

    /// Set an environment variable value for a session
    pub fn set_environment(session_name: &str, var: &str, value: &str) -> Result<()> {
        TmuxMultiplexer::set_environment(session_name, var, value)
    }

    /// Respawn the primary pane with a new shell script
    pub fn respawn_pane(session_name: &str, script: &str) -> Result<()> {
        TmuxMultiplexer::respawn_pane(session_name, script)
    }

    /// Kill a session
    pub fn kill_session(&self, name: &str) -> Result<()> {
        self.inner.kill_session(name)
    }

    /// Rename a session
    pub fn rename_session(&self, old_name: &str, new_display_name: &str) -> Result<String> {
        self.inner.rename_session(old_name, new_display_name)
    }

    /// Fork a session (creates a new one with same content)
    pub fn fork_session(&self, original_name: &str, new_display_name: &str) -> Result<String> {
        self.inner.fork_session(original_name, new_display_name)
    }

    /// Get pane content (last N lines)
    pub fn get_pane_content(session_name: &str, lines: usize) -> Result<String> {
        TmuxMultiplexer::get_pane_content(session_name, lines)
    }

    /// Get window activity timestamp
    pub fn get_window_activity(&self, session_name: &str) -> Option<i64> {
        self.inner.get_window_activity(session_name)
    }

    /// Detect current terminal
    pub fn detect_terminal() -> TerminalInfo {
        TmuxMultiplexer::detect_terminal()
    }

    /// List all maestro sessions
    pub fn list_maestro_sessions(&self) -> Vec<String> {
        self.inner.list_maestro_sessions()
    }

    /// Get all current pane paths from all windows across all sessions
    pub fn get_all_pane_paths(&self) -> Result<Vec<String>> {
        self.inner.get_all_pane_paths()
    }

    /// Get the active pane's current path (if available)
    pub fn get_active_pane_path(&self) -> Result<Option<String>> {
        self.inner.get_active_pane_path()
    }

    // ========== Tab-rs Integration Hooks ==========
    // These methods will be implemented in Phase 2 to interface with tab-rs

    /// Check if tab-rs binary is available (for Phase 2)
    pub fn is_tab_rs_available() -> bool {
        Command::new("tab")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get tab-rs version information (for Phase 2)
    pub fn tab_rs_version() -> Option<String> {
        let output = Command::new("tab").arg("--version").output().ok()?;
        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }
}

// Re-export the helper functions for external use
pub use super::tmux::{sanitize_name, shell_quote};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_conversion() {
        let tmux = TmuxSession::new("Test Session", "/tmp/test");
        let maestro: MaestroTabSession = tmux.clone().into();
        assert_eq!(maestro.display_name, "Test Session");
        assert_eq!(maestro.work_dir, "/tmp/test");

        let back: TmuxSession = maestro.into();
        assert_eq!(back.display_name, "Test Session");
    }

    #[test]
    fn test_status_conversion() {
        use super::super::tmux::TmuxSessionStatus;
        use MaestroTabSessionStatus::*;

        let tests = [
            (TmuxSessionStatus::Active, Active),
            (TmuxSessionStatus::Waiting, Waiting),
            (TmuxSessionStatus::Idle, Idle),
            (TmuxSessionStatus::Error, Error),
        ];

        for (tmux_status, expected) in tests {
            let maestro_status: MaestroTabSessionStatus = tmux_status.into();
            assert_eq!(maestro_status, expected);

            let back: TmuxSessionStatus = maestro_status.into();
            assert_eq!(back, tmux_status);
        }
    }
}
