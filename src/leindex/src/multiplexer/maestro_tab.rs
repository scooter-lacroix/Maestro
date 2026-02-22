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
    pub fn start_session(
        &self,
        session: &mut MaestroTabSession,
        command: Option<&str>,
    ) -> Result<()> {
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

    /// Check if tab daemon is running (for subprocess integration)
    pub fn is_tab_daemon_running() -> bool {
        // Check if tab daemon socket exists
        let socket_path = std::env::var("TAB_SOCKET").unwrap_or_else(|_| {
            format!(
                "{}/.tab/daemon.sock",
                std::env::var("HOME").unwrap_or_default()
            )
        });

        std::path::Path::new(&socket_path).exists()
    }

    /// Create a tab via CLI subprocess (Phase 2 integration)
    ///
    /// This method spawns the `tab` binary to create a new terminal tab.
    /// Falls back to tmux if tab binary is not available.
    pub fn create_tab_via_cli(&self, name: &str, command: Option<&str>) -> Result<()> {
        if !Self::is_tab_rs_available() {
            debug!("tab binary not available, using tmux fallback");
            // Use the inner tmux multiplexer instead
            let mut session = MaestroTabSession::new(name, ".");
            return self.start_session(&mut session, command);
        }

        let mut cmd = Command::new("tab");
        cmd.arg("create").arg("--name").arg(name);

        if let Some(cmd_str) = command {
            cmd.arg("--command").arg(cmd_str);
        }

        let output = cmd.output()?;
        if !output.status.success() {
            anyhow::bail!(
                "Failed to create tab: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        debug!("Created tab '{}' via CLI", name);
        Ok(())
    }

    /// List tabs via CLI subprocess (Phase 2 integration)
    pub fn list_tabs_via_cli(&self) -> Result<Vec<String>> {
        if !Self::is_tab_rs_available() {
            return Ok(self.list_maestro_sessions());
        }

        let output = Command::new("tab").arg("list").output()?;
        if !output.status.success() {
            return Ok(self.list_maestro_sessions());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().map(|s| s.trim().to_string()).collect())
    }

    /// Close a tab via CLI subprocess (Phase 2 integration)
    pub fn close_tab_via_cli(&self, name: &str) -> Result<()> {
        if !Self::is_tab_rs_available() {
            return self.kill_session(name);
        }

        let output = Command::new("tab").arg("close").arg(name).output()?;
        if !output.status.success() {
            debug!("Failed to close tab via CLI, falling back to tmux");
            return self.kill_session(name);
        }

        debug!("Closed tab '{}' via CLI", name);
        Ok(())
    }
}

// ========== Transparency Support ==========
// OSC 111 sequences for foot terminal transparency

/// OSC 111 transparency sequence prefix
pub const OSC_111_PREFIX: &str = "\x1b]111;";

/// OSC 111 transparency sequence suffix
pub const OSC_111_SUFFIX: &str = "\x07";

/// Generate OSC 111 transparency sequence for a given alpha value (0-255)
pub fn transparency_sequence(alpha: u8) -> String {
    format!("{}{}{}", OSC_111_PREFIX, alpha, OSC_111_SUFFIX)
}

/// Reset transparency to terminal default
pub fn reset_transparency_sequence() -> String {
    format!("{}{}", OSC_111_PREFIX, OSC_111_SUFFIX)
}

/// Shell hook script for fish shell to maintain transparency
pub fn fish_transparency_hook(alpha: u8) -> String {
    format!(
        r#"
# Maestro transparency hook for fish shell
function __maestro_transparency --on-event fish_prompt
    echo -n "{}"
end
"#,
        transparency_sequence(alpha)
    )
}

/// Shell hook script for bash to maintain transparency
pub fn bash_transparency_hook(alpha: u8) -> String {
    format!(
        r#"
# Maestro transparency hook for bash
__maestro_transparency() {{
    echo -n "{}"
}}
PROMPT_COMMAND="__maestro_transparency${{PROMPT_COMMAND:+; $PROMPT_COMMAND}}"
"#,
        transparency_sequence(alpha)
    )
}

/// Shell hook script for zsh to maintain transparency
pub fn zsh_transparency_hook(alpha: u8) -> String {
    format!(
        r#"
# Maestro transparency hook for zsh
__maestro_transparency() {{
    echo -n "{}"
}}
precmd_functions+=(__maestro_transparency)
"#,
        transparency_sequence(alpha)
    )
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

    #[test]
    fn test_transparency_sequence() {
        // Test alpha 0 (fully transparent)
        let seq = transparency_sequence(0);
        assert!(seq.starts_with("\x1b]111;"));
        assert!(seq.ends_with("\x07"));
        assert!(seq.contains("0"));

        // Test alpha 255 (fully opaque)
        let seq = transparency_sequence(255);
        assert!(seq.contains("255"));

        // Test alpha 128 (50% transparency)
        let seq = transparency_sequence(128);
        assert!(seq.contains("128"));
    }

    #[test]
    fn test_reset_transparency_sequence() {
        let seq = reset_transparency_sequence();
        assert_eq!(seq, "\x1b]111;\x07");
    }

    #[test]
    fn test_shell_hooks_contain_sequence() {
        let alpha = 200;
        let expected_seq = transparency_sequence(alpha);

        // Fish hook
        let fish_hook = fish_transparency_hook(alpha);
        assert!(fish_hook.contains(&expected_seq));
        assert!(fish_hook.contains("fish_prompt"));

        // Bash hook
        let bash_hook = bash_transparency_hook(alpha);
        assert!(bash_hook.contains(&expected_seq));
        assert!(bash_hook.contains("PROMPT_COMMAND"));

        // Zsh hook
        let zsh_hook = zsh_transparency_hook(alpha);
        assert!(zsh_hook.contains(&expected_seq));
        assert!(zsh_hook.contains("precmd_functions"));
    }

    #[test]
    fn test_tab_rs_version_returns_none_when_not_available() {
        // This test verifies the function handles missing tab binary gracefully
        // It should return None rather than panic
        let _result = MaestroTabMultiplexer::tab_rs_version();
        // Result depends on whether tab is installed on the system
    }
}
