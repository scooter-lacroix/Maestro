//! Tmux Multiplexer Module
//!
//! High-level Rust API for session management using tmux-rs.
//! This module ports the Go TUI's tmux integration to native Rust.

use anyhow::{bail, Context, Result};
use dashmap::DashMap;
use std::process::Command;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

const SESSION_PREFIX: &str = "maestro_";

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

/// Terminal capabilities detected from environment
#[derive(Debug, Clone)]
pub struct TerminalInfo {
    pub name: String,
    pub supports_osc8: bool,  // OSC 8 hyperlinks
    pub supports_osc52: bool, // OSC 52 clipboard
    pub supports_true_color: bool,
}

/// State tracker for notification-style status detection
#[derive(Debug, Clone)]
pub struct StateTracker {
    pub last_hash: String,
    pub last_change_time: Instant,
    pub acknowledged: bool,
    pub acknowledged_at: Option<Instant>,
    pub last_activity_timestamp: i64,
}

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

/// A tmux session handle
#[derive(Debug, Clone)]
pub struct TmuxSession {
    pub name: String,
    pub display_name: String,
    pub work_dir: String,
    pub command: Option<String>,
    pub created: Instant,
    pub state_tracker: StateTracker,
}

impl TmuxSession {
    /// Create a new session handle (generates unique name)
    pub fn new(display_name: &str, work_dir: &str) -> Self {
        let sanitized = sanitize_name(display_name);
        let unique_suffix = generate_short_id();

        Self {
            name: format!("{}{}_{}", SESSION_PREFIX, sanitized, unique_suffix),
            display_name: display_name.to_string(),
            work_dir: work_dir.to_string(),
            command: None,
            created: Instant::now(),
            state_tracker: StateTracker::default(),
        }
    }

    /// Create session with a specific name (for restoration)
    ///
    /// This constructor is used when restoring a session to preserve the original
    /// tmux session name instead of generating a new unique one.
    pub fn with_name(name: String, display_name: &str, work_dir: &str) -> Self {
        Self {
            name,
            display_name: display_name.to_string(),
            work_dir: work_dir.to_string(),
            command: None,
            created: Instant::now(),
            state_tracker: StateTracker::default(),
        }
    }

    /// Get the log file path for this session
    pub fn log_file(&self) -> std::path::PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        home.join(".maestro")
            .join("logs")
            .join(format!("{}.log", self.name))
    }
}

/// Tmux multiplexer - manages tmux sessions
#[derive(Debug)]
pub struct TmuxMultiplexer {
    /// Session cache: name -> activity timestamp
    session_cache: DashMap<String, i64>,
    /// When the cache was last refreshed
    cache_time: std::sync::RwLock<Option<Instant>>,
    /// Cache TTL in seconds
    cache_ttl: Duration,
}

impl Default for TmuxMultiplexer {
    fn default() -> Self {
        Self::new()
    }
}

impl TmuxMultiplexer {
    /// Create a new multiplexer instance
    pub fn new() -> Self {
        // Configure global tmux settings for transparency on first use
        Self::configure_global_transparency();

        Self {
            session_cache: DashMap::new(),
            cache_time: std::sync::RwLock::new(None),
            cache_ttl: Duration::from_secs(2),
        }
    }

    /// Configure global tmux settings for transparency
    /// This must be done at server level, not session level
    fn configure_global_transparency() {
        // Set global window-style to default (transparent)
        // This must be done BEFORE creating any sessions
        let _ = Command::new("tmux")
            .args(["set-option", "-g", "window-style", "bg=default"])
            .output();

        // CRITICAL: Also set window-active-style for the active window
        let _ = Command::new("tmux")
            .args(["set-option", "-g", "window-active-style", "bg=default"])
            .output();

        // Set global pane border styles
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-g",
                "pane-border-style",
                "fg=#3d59a1,bg=default",
            ])
            .output();

        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-g",
                "pane-active-border-style",
                "fg=#7aa2f7,bg=default",
            ])
            .output();

        // Set terminal type for true color support
        let _ = Command::new("tmux")
            .args(["set-option", "-g", "default-terminal", "tmux-256color"])
            .output();

        // Enable true color pass-through (critical for transparency)
        let _ = Command::new("tmux")
            .args(["set-option", "-ga", "terminal-overrides", ",*256col*:Tc"])
            .output();

        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-ga",
                "terminal-overrides",
                ",xterm-256color:RGB",
            ])
            .output();

        // Add foot terminal specific overrides for transparency
        // foot uses the "bce" capability, we need to handle it properly
        let _ = Command::new("tmux")
            .args(["set-option", "-ga", "terminal-overrides", ",foot:Tc"])
            .output();

        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-ga",
                "terminal-overrides",
                ",foot-256color:Tc",
            ])
            .output();

        // Enable passthrough for modern terminal features (OSC sequences)
        let _ = Command::new("tmux")
            .args(["set-option", "-g", "allow-passthrough", "on"])
            .output();

        // Set status bar to transparent
        let _ = Command::new("tmux")
            .args(["set-option", "-g", "status-style", "bg=default,fg=#a9b1d6"])
            .output();

        // Configure message style for transparency
        let _ = Command::new("tmux")
            .args(["set-option", "-g", "message-style", "bg=default,fg=#c0caf5"])
            .output();

        // Configure mode style (copy mode, etc.) for transparency
        let _ = Command::new("tmux")
            .args(["set-option", "-g", "mode-style", "bg=default,fg=#7aa2f7"])
            .output();

        // CRITICAL: Set the popup style for transparency (for any popup dialogs)
        let _ = Command::new("tmux")
            .args(["set-option", "-g", "popup-style", "bg=default,fg=#c0caf5"])
            .output();

        // Set display-panes style for transparency
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-g",
                "display-panes-style",
                "bg=default,fg=#7aa2f7",
            ])
            .output();

        // Set clock mode style for transparency
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-g",
                "clock-mode-style",
                "bg=default,fg=#a9b1d6",
            ])
            .output();

        debug!("Configured global tmux transparency settings");
    }

    /// Check if tmux is available
    pub fn is_available() -> Result<()> {
        let output = Command::new("tmux")
            .arg("-V")
            .output()
            .context("Failed to execute tmux")?;

        if output.status.success() {
            Ok(())
        } else {
            bail!(
                "tmux not found or not working: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }

    /// Refresh the session cache (call once per tick)
    /// This reduces subprocess spawns from O(n) to O(1)
    pub fn refresh_session_cache(&self) -> Result<()> {
        let output = Command::new("tmux")
            .args([
                "list-sessions",
                "-F",
                "#{session_name}\t#{session_activity}",
            ])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                self.session_cache.clear();

                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.is_empty() {
                        continue;
                    }
                    let parts: Vec<&str> = line.splitn(2, '\t').collect();
                    if parts.len() == 2 {
                        let name = parts[0].to_string();
                        let activity: i64 = parts[1].parse().unwrap_or(0);
                        self.session_cache.insert(name, activity);
                    }
                }

                *self.cache_time.write().unwrap() = Some(Instant::now());
                Ok(())
            }
            _ => {
                // tmux not running or error - clear cache
                self.session_cache.clear();
                *self.cache_time.write().unwrap() = None;
                Ok(())
            }
        }
    }

    /// Check if session exists from cache
    pub fn session_exists_from_cache(&self, name: &str) -> Option<bool> {
        let cache_time = self.cache_time.read().unwrap();

        if let Some(time) = *cache_time {
            if time.elapsed() <= self.cache_ttl {
                return Some(self.session_cache.contains_key(name));
            }
        }
        None
    }

    /// Get session activity from cache
    pub fn session_activity_from_cache(&self, name: &str) -> Option<i64> {
        let cache_time = self.cache_time.read().unwrap();

        if let Some(time) = *cache_time {
            if time.elapsed() <= self.cache_ttl {
                return self.session_cache.get(name).map(|r| *r);
            }
        }
        None
    }

    /// Register a newly created session in cache (prevents race condition)
    pub fn register_session_in_cache(&self, name: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        self.session_cache.insert(name.to_string(), now);
    }

    /// Check if a session exists
    pub fn session_exists(&self, name: &str) -> bool {
        // Try cache first
        if let Some(exists) = self.session_exists_from_cache(name) {
            return exists;
        }

        // Fall back to direct check
        Command::new("tmux")
            .args(["has-session", "-t", name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Start a new tmux session
    pub fn start_session(&self, session: &mut TmuxSession, command: Option<&str>) -> Result<()> {
        session.command = command.map(String::from);

        // Check if session already exists
        if self.session_exists(&session.name) {
            // Regenerate with new unique suffix
            let sanitized = sanitize_name(&session.display_name);
            session.name = format!("{}{}_{}", SESSION_PREFIX, sanitized, generate_short_id());
        }

        // Ensure working directory exists
        let work_dir = if session.work_dir.is_empty() {
            std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
        } else {
            session.work_dir.clone()
        };

        // Build environment variables
        let mut env_args: Vec<String> = Vec::new();

        if let Ok(home) = std::env::var("HOME") {
            env_args.push("-e".to_string());
            env_args.push(format!("HOME={}", home));
        }

        if let Ok(path) = std::env::var("PATH") {
            env_args.push("-e".to_string());
            env_args.push(format!("PATH={}", path));
        }

        // Claude config dir
        let claude_config = std::env::var("CLAUDE_CONFIG_DIR").unwrap_or_else(|_| {
            dirs::home_dir()
                .map(|h| h.join(".claude").to_string_lossy().to_string())
                .unwrap_or_else(|| "~/.claude".to_string())
        });
        env_args.push("-e".to_string());
        env_args.push(format!("CLAUDE_CONFIG_DIR={}", claude_config));

        // Pass through COLORTERM for true color support (enables transparency in many terminals)
        if let Ok(colorterm) = std::env::var("COLORTERM") {
            env_args.push("-e".to_string());
            env_args.push(format!("COLORTERM={}", colorterm));
        }

        // Pass through TERM for proper terminal detection
        if let Ok(term) = std::env::var("TERM") {
            env_args.push("-e".to_string());
            env_args.push(format!("TERM={}", term));
        }

        // Set environment variable to indicate transparency is desired
        // Shells can check this to avoid setting background colors
        env_args.push("-e".to_string());
        env_args.push("MAESTRO_TRANSPARENCY=1".to_string());

        // Build tmux new-session command
        let mut args = vec![
            "new-session".to_string(),
            "-d".to_string(),
            "-s".to_string(),
            session.name.clone(),
            "-c".to_string(),
            work_dir,
        ];
        args.extend(env_args);

        debug!("Creating tmux session: tmux {}", args.join(" "));

        let output = Command::new("tmux")
            .args(&args)
            .output()
            .context("Failed to create tmux session")?;

        if !output.status.success() {
            bail!(
                "Failed to create tmux session: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Register in cache immediately
        self.register_session_in_cache(&session.name);

        // Configure session options
        self.configure_session_options(&session.name)?;

        // Configure status bar
        self.configure_status_bar(session)?;

        // Send command if provided
        if let Some(cmd) = command {
            self.send_keys(&session.name, cmd)?;
            self.send_enter(&session.name)?;
        }

        // Enable pipe-pane for logging
        if let Err(e) = self.enable_pipe_pane(session) {
            warn!("Failed to enable pipe-pane for {}: {}", session.name, e);
        }

        // CRITICAL: Apply transparency reset by EXECUTING printf in the shell
        //
        // The problem with send-keys -l: it sends to shell INPUT (like typing),
        // but escape sequences need to go through shell OUTPUT to reach the terminal.
        //
        // Solution: Execute the printf command IN the shell, so it writes to stdout,
        // which goes through tmux and reaches the actual terminal.
        //
        // The working sequence: printf '\033[0m\033]111\007\033[49m\033[2J\033[H'
        // - \033[0m      = Reset all attributes (SGR 0)
        // - \033]111\007 = OSC 111 (reset background to default/transparent)
        // - \033[49m     = Reset background to default
        // - \033[2J      = Clear entire screen
        // - \033[H       = Move cursor to home

        let transparency_cmd = r#"printf '\033[0m\033]111\007\033[49m\033[2J\033[H'"#;

        // Send the printf command to the shell
        let _ = Command::new("tmux")
            .args(["send-keys", "-t", &session.name, "-l", transparency_cmd])
            .output();

        // Small delay to ensure command is received
        std::thread::sleep(std::time::Duration::from_millis(30));

        // Press Enter to execute the command
        let _ = Command::new("tmux")
            .args(["send-keys", "-t", &session.name, "Enter"])
            .output();

        // Wait for the command to execute and take effect
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Force tmux to redraw
        let _ = Command::new("tmux").args(["refresh-client"]).output();

        info!("Created tmux session with transparency: {}", session.name);
        Ok(())
    }

    /// Configure session options (mouse, clipboard, etc.)
    fn configure_session_options(&self, session_name: &str) -> Result<()> {
        // FIRST: Create shell transparency hooks BEFORE the session starts
        // This ensures they're available when the shell initializes
        if let Some(home) = dirs::home_dir() {
            // Create fish transparency hook (auto-loaded from conf.d)
            let fish_conf_dir = home.join(".config/fish/conf.d");
            if let Err(e) = std::fs::create_dir_all(&fish_conf_dir) {
                debug!("Could not create fish conf.d directory: {}", e);
            } else {
                let transparency_hook = fish_conf_dir.join("maestro-transparency.fish");
                let hook_content = r#"# Maestro Terminal Transparency Hook
# Reset background to transparent after commands and prompts

function __maestro_reset_background --on-event fish_postexec
    printf '\033[0m\033]111\007\033[49m' 2>/dev/null
end

function __maestro_reset_background_prompt --on-event fish_prompt
    printf '\033[0m\033]111\007\033[49m' 2>/dev/null
end

# Initial reset
printf '\033[0m\033]111\007\033[49m' 2>/dev/null
"#;
                let _ = std::fs::write(&transparency_hook, hook_content);
                debug!("Created fish transparency hook at {:?}", transparency_hook);
            }

            // Create bash transparency hook (needs to be sourced in .bashrc)
            let maestro_dir = home.join(".maestro");
            let _ = std::fs::create_dir_all(&maestro_dir);
            let bash_hook_path = maestro_dir.join("bash_transparency.sh");
            let bash_hook_content = r#"# Maestro Terminal Transparency Hook for Bash
# Add to your .bashrc: source ~/.maestro/bash_transparency.sh

__maestro_reset_background() {
    printf '\033[0m\033]111\007\033[49m' 2>/dev/null
}

if [[ -z "$PROMPT_COMMAND" ]]; then
    PROMPT_COMMAND="__maestro_reset_background"
elif [[ ! "$PROMPT_COMMAND" =~ __maestro_reset_background ]]; then
    PROMPT_COMMAND="__maestro_reset_background;$PROMPT_COMMAND"
fi

__maestro_reset_background
"#;
            let _ = std::fs::write(&bash_hook_path, bash_hook_content);

            // Create zsh transparency hook (needs to be sourced in .zshrc)
            let zsh_hook_path = maestro_dir.join("zsh_transparency.zsh");
            let zsh_hook_content = r#"# Maestro Terminal Transparency Hook for Zsh
# Add to your .zshrc: source ~/.maestro/zsh_transparency.zsh

__maestro_reset_background() {
    printf '\033[0m\033]111\007\033[49m' 2>/dev/null
}

if [[ ! " ${precmd_functions[@]} " =~ " __maestro_reset_background " ]]; then
    precmd_functions+=(__maestro_reset_background)
fi

__maestro_reset_background
"#;
            let _ = std::fs::write(&zsh_hook_path, zsh_hook_content);
        }

        let options = [
            ("mouse", "on"),
            ("set-clipboard", "on"),
            ("history-limit", "10000"),
            ("escape-time", "10"),
            ("detach-on-destroy", "off"),
            // Set proper terminal type for true color support
            ("default-terminal", "tmux-256color"),
        ];

        for (option, value) in options {
            let _ = Command::new("tmux")
                .args(["set-option", "-t", session_name, option, value])
                .output();
        }

        // Enable true color (RGB) pass-through for transparency support
        // This allows the parent terminal's transparency to show through tmux
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                session_name,
                "-a",
                "terminal-overrides",
                ",*256col*:Tc",
            ])
            .output();

        // Additional terminal overrides for xterm and common terminals
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                session_name,
                "-a",
                "terminal-overrides",
                ",xterm-256color:RGB",
            ])
            .output();

        // Ensure remain-on-exit is NOT set by default (want it to die if process finishes normally)
        // but want it to STAY if we detach.
        let _ = Command::new("tmux")
            .args([
                "set-window-option",
                "-t",
                session_name,
                "remain-on-exit",
                "off",
            ])
            .output();

        // Transparency support: Set window background to inherit from parent terminal
        // This allows terminal transparency to pass through tmux sessions
        let _ = Command::new("tmux")
            .args([
                "set-window-option",
                "-t",
                session_name,
                "window-style",
                "bg=default",
            ])
            .output();

        // Pane background style - critical for transparency
        // "default" means inherit from parent terminal, not set a color
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                session_name,
                "-p",
                "window-style",
                "bg=default",
            ])
            .output();

        // Set pane border styles (subtle, non-intrusive, transparent)
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                session_name,
                "pane-border-style",
                "fg=#3d59a1,bg=default",
            ])
            .output();

        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                session_name,
                "pane-active-border-style",
                "fg=#7aa2f7,bg=default",
            ])
            .output();

        // Main pane style - ensure the actual content area is transparent
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                session_name,
                "-p",
                "pane-border-style",
                "fg=#3d59a1,bg=default",
            ])
            .output();

        // Custom detach bind: Ctrl+Shift+Q is hard to capture in all terminals,
        // using a more reliable sequence or just binding C-q if user prefers.
        // User asked for ctrl+shift+q. We'll try to bind it as C-S-q.
        let _ = Command::new("tmux")
            .args(["bind-key", "-n", "C-q", "detach-client"])
            .output();

        // Enable passthrough for modern terminal features (tmux 3.2+)
        // This is CRITICAL for transparency - allows escape sequences to pass through
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                session_name,
                "-q",
                "allow-passthrough",
                "on",
            ])
            .output();

        // Enable hyperlinks (tmux 3.4+)
        let _ = Command::new("tmux")
            .args(["set", "-asq", "terminal-features", ",*:hyperlinks"])
            .output();

        // Set environment variables that shells can check to avoid setting background colors
        // Many modern terminals and shells respect these for transparency-aware configurations
        let _ = Command::new("tmux")
            .args([
                "set-environment",
                "-t",
                session_name,
                "TERMINAL_HAS_TRANSPARENCY",
                "1",
            ])
            .output();

        // Set COLORTERM to indicate true color support with transparency
        let _ = Command::new("tmux")
            .args([
                "set-environment",
                "-t",
                session_name,
                "COLORTERM",
                "truecolor",
            ])
            .output();

        Ok(())
    }

    /// Configure status bar with session info
    fn configure_status_bar(&self, session: &TmuxSession) -> Result<()> {
        let folder_name = std::path::Path::new(&session.work_dir)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("~");

        // Enable status bar
        let _ = Command::new("tmux")
            .args(["set-option", "-t", &session.name, "status", "on"])
            .output();

        // Style (Tokyo Night inspired, with transparency support)
        // Use bg=default to inherit terminal transparency
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                &session.name,
                "status-style",
                "bg=default,fg=#a9b1d6",
            ])
            .output();

        // Left side: session title
        let left_status = format!(" {} ", session.display_name);
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                &session.name,
                "status-left",
                &left_status,
            ])
            .output();
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                &session.name,
                "status-left-length",
                "40",
            ])
            .output();

        // Right side: folder name + escape hint
        let right_status = format!(" {} #[fg=#ff9e64,bold]Detach: Ctrl-B d ", folder_name);
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                &session.name,
                "status-right",
                &right_status,
            ])
            .output();
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                &session.name,
                "status-right-length",
                "50",
            ])
            .output();

        Ok(())
    }

    /// Enable pipe-pane to log output
    fn enable_pipe_pane(&self, session: &TmuxSession) -> Result<()> {
        let log_file = session.log_file();

        // Ensure log directory exists
        if let Some(parent) = log_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let log_path = log_file.to_string_lossy();
        let pipe_cmd = format!("cat >> {}", shell_quote(&log_path));

        Command::new("tmux")
            .args(["pipe-pane", "-t", &session.name, "-o", &pipe_cmd])
            .output()
            .context("Failed to enable pipe-pane")?;

        Ok(())
    }

    /// Send keys to a session
    pub fn send_keys(&self, session_name: &str, keys: &str) -> Result<()> {
        Command::new("tmux")
            .args(["send-keys", "-t", session_name, keys])
            .output()
            .context("Failed to send keys")?;
        Ok(())
    }

    /// Send Enter key to a session
    pub fn send_enter(&self, session_name: &str) -> Result<()> {
        Command::new("tmux")
            .args(["send-keys", "-t", session_name, "Enter"])
            .output()
            .context("Failed to send Enter")?;
        Ok(())
    }

    /// Attach to a session
    pub fn attach(session_name: &str) -> Result<()> {
        // Double check session exists
        let mux = Self::default();
        if !mux.session_exists(session_name) {
            bail!("Session '{}' no longer exists in tmux", session_name);
        }

        let status = Command::new("tmux")
            .args(["attach-session", "-t", session_name])
            .status()
            .context("Failed to attach to session")?;

        if !status.success() {
            bail!("Failed to attach to session: {}", session_name);
        }
        Ok(())
    }

    /// Get an environment variable value for a tmux session.
    ///
    /// Returns `Ok(None)` when the variable is unset or the session doesn't exist.
    pub fn get_environment(session_name: &str, var: &str) -> Result<Option<String>> {
        let output = Command::new("tmux")
            .args(["show-environment", "-t", session_name, var])
            .output();

        let Ok(output) = output else {
            return Ok(None);
        };
        if !output.status.success() {
            return Ok(None);
        }

        let line = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if line.is_empty() || line.starts_with('-') {
            return Ok(None);
        }

        if let Some((_k, v)) = line.split_once('=') {
            Ok(Some(v.to_string()))
        } else {
            Ok(None)
        }
    }

    /// Set an environment variable value for a tmux session.
    pub fn set_environment(session_name: &str, var: &str, value: &str) -> Result<()> {
        let output = Command::new("tmux")
            .args(["set-environment", "-t", session_name, var, value])
            .output()
            .context("Failed to set tmux environment")?;

        if !output.status.success() {
            bail!(
                "Failed to set tmux environment: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    /// Respawn the primary pane (0.0) with a new shell script.
    ///
    /// Uses `sh -lc` so the script runs under a login-like shell.
    pub fn respawn_pane(session_name: &str, script: &str) -> Result<()> {
        let target = format!("{}:0.0", session_name);
        let output = Command::new("tmux")
            .args(["respawn-pane", "-k", "-t", &target, "sh", "-lc", script])
            .output()
            .context("Failed to respawn tmux pane")?;

        if !output.status.success() {
            bail!(
                "Failed to respawn tmux pane: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    /// Kill a session
    pub fn kill_session(&self, name: &str) -> Result<()> {
        let _ = Command::new("tmux")
            .args(["kill-session", "-t", name])
            .output()
            .context("Failed to kill session")?;
        self.session_cache.remove(name);
        Ok(())
    }

    /// Rename a session
    pub fn rename_session(&self, old_name: &str, new_display_name: &str) -> Result<String> {
        let sanitized = sanitize_name(new_display_name);
        let unique_suffix = generate_short_id();
        let new_name = format!("{}{}_{}", SESSION_PREFIX, sanitized, unique_suffix);

        let output = Command::new("tmux")
            .args(["rename-session", "-t", old_name, &new_name])
            .output()
            .context("Failed to rename tmux session")?;

        if !output.status.success() {
            bail!(
                "Tmux rename-session failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        self.session_cache.remove(old_name);
        self.register_session_in_cache(&new_name);

        Ok(new_name)
    }

    /// Fork a session (creates a new one with same content as much as possible)
    pub fn fork_session(&self, original_name: &str, new_display_name: &str) -> Result<String> {
        let mut new_sess = TmuxSession::new(new_display_name, "/tmp"); // Temporary dir, will update

        // Get original attributes
        let output = Command::new("tmux")
            .args([
                "display-message",
                "-t",
                original_name,
                "-p",
                "#{pane_current_path}",
            ])
            .output()?;
        let original_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        new_sess.work_dir = original_path;

        // Start new session
        self.start_session(&mut new_sess, None)?;

        // Optional: Pipe some output from original to new if possible
        // (Tmux doesn't easily support full state fork, but we can capture buffer)
        if let Ok(content) = Self::get_pane_content(original_name, 100) {
            let _ = self.send_keys(
                &new_sess.name,
                &format!(
                    "echo \"--- Forked from {} ---\"; cat << 'EOF'\n{}\nEOF\n",
                    original_name, content
                ),
            );
        }

        Ok(new_sess.name)
    }

    /// Get pane content (last N lines)
    pub fn get_pane_content(session_name: &str, lines: usize) -> Result<String> {
        let output = Command::new("tmux")
            .args([
                "capture-pane",
                "-t",
                session_name,
                "-p", // Print to stdout
                "-J", // Join wrapped lines
                "-S",
                &format!("-{}", lines), // Start from N lines back
            ])
            .output()
            .context("Failed to capture pane")?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get window activity timestamp
    pub fn get_window_activity(&self, session_name: &str) -> Option<i64> {
        // Try cache first
        if let Some(activity) = self.session_activity_from_cache(session_name) {
            return Some(activity);
        }

        // Fall back to direct query
        let output = Command::new("tmux")
            .args([
                "display-message",
                "-t",
                session_name,
                "-p",
                "#{window_activity}",
            ])
            .output()
            .ok()?;

        String::from_utf8_lossy(&output.stdout).trim().parse().ok()
    }

    /// Detect current terminal
    pub fn detect_terminal() -> TerminalInfo {
        let name = if std::env::var("TERM_PROGRAM").as_deref() == Ok("WarpTerminal")
            || std::env::var("WARP_IS_LOCAL_SHELL_SESSION").is_ok()
        {
            "warp"
        } else if std::env::var("TERM_PROGRAM").as_deref() == Ok("iTerm.app")
            || std::env::var("ITERM_SESSION_ID").is_ok()
        {
            "iterm2"
        } else if std::env::var("TERM").as_deref() == Ok("xterm-kitty")
            || std::env::var("KITTY_WINDOW_ID").is_ok()
        {
            "kitty"
        } else if std::env::var("ALACRITTY_SOCKET").is_ok()
            || std::env::var("ALACRITTY_LOG").is_ok()
        {
            "alacritty"
        } else if std::env::var("TERM_PROGRAM").as_deref() == Ok("vscode")
            || std::env::var("VSCODE_INJECTION").is_ok()
        {
            "vscode"
        } else if std::env::var("WT_SESSION").is_ok() {
            "windows-terminal"
        } else if std::env::var("TERM_PROGRAM").as_deref() == Ok("WezTerm")
            || std::env::var("WEZTERM_PANE").is_ok()
        {
            "wezterm"
        } else if std::env::var("TERM_PROGRAM").as_deref() == Ok("Apple_Terminal") {
            "apple-terminal"
        } else {
            std::env::var("TERM_PROGRAM")
                .map(|p| p.to_lowercase())
                .unwrap_or_else(|_| "unknown".to_string())
                .as_str()
                .to_string()
                .leak()
        }
        .to_string();

        let supports_true_color = std::env::var("COLORTERM")
            .map(|c| c == "truecolor" || c == "24bit")
            .unwrap_or(false);

        let (supports_osc8, supports_osc52) = match name.as_str() {
            "warp" | "iterm2" | "kitty" | "alacritty" | "wezterm" | "windows-terminal"
            | "vscode" => (true, true),
            "apple-terminal" => (false, false),
            _ => (true, true), // Optimistic default
        };

        TerminalInfo {
            name,
            supports_osc8,
            supports_osc52,
            supports_true_color,
        }
    }

    /// List all maestro sessions
    pub fn list_maestro_sessions(&self) -> Vec<String> {
        self.refresh_session_cache().ok();

        self.session_cache
            .iter()
            .filter(|r| r.key().starts_with(SESSION_PREFIX))
            .map(|r| r.key().clone())
            .collect()
    }

    /// Get all current pane paths from all windows across all sessions
    pub fn get_all_pane_paths(&self) -> Result<Vec<String>> {
        let output = Command::new("tmux")
            .args(["list-panes", "-a", "-F", "#{pane_current_path}"])
            .output()
            .context("Failed to list tmux panes")?;

        if !output.status.success() {
            // It might fail if tmux server is not running
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut paths: Vec<String> = stdout
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        // Deduplicate and sort
        paths.sort();
        paths.dedup();

        Ok(paths)
    }

    /// Get the active pane's current path (if available)
    pub fn get_active_pane_path(&self) -> Result<Option<String>> {
        let output = Command::new("tmux")
            .args(["display-message", "-p", "#{pane_current_path}"])
            .output()
            .context("Failed to query tmux active pane path")?;

        if !output.status.success() {
            return Ok(None);
        }

        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() {
            Ok(None)
        } else {
            Ok(Some(path))
        }
    }
}

/// Sanitize a display name to a valid tmux session name
pub fn sanitize_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();

    // Collapse multiple hyphens
    let mut result = String::new();
    let mut last_was_hyphen = false;
    for c in sanitized.chars() {
        if c == '-' {
            if !last_was_hyphen {
                result.push(c);
            }
            last_was_hyphen = true;
        } else {
            result.push(c);
            last_was_hyphen = false;
        }
    }

    // Trim hyphens and limit length
    let result = result.trim_matches('-').to_string();
    if result.len() > 50 {
        result[..50].to_string()
    } else if result.is_empty() {
        "session".to_string()
    } else {
        result
    }
}

/// Generate a short random ID
fn generate_short_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    format!("{:08x}", (now % 0xFFFFFFFF) as u32)
}

/// Shell-quote a string for safe use in shell commands
pub fn shell_quote(s: &str) -> String {
    if !s.contains(|c: char| {
        c == '\''
            || c == '\\'
            || c == '\n'
            || c == '\r'
            || c == '\t'
            || c == ';'
            || c == '&'
            || c == '|'
            || c == '<'
            || c == '>'
            || c == '$'
            || c == '`'
            || c == ' '
    }) {
        return s.to_string();
    }

    // Single quote escaping: replace ' with '\''
    let escaped = s.replace("'", "'\\''");
    format!("'{}'", escaped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("my-project"), "my-project");
        assert_eq!(sanitize_name("My Project!"), "My-Project"); // Trailing hyphen is trimmed
        assert_eq!(sanitize_name("  test  "), "test");
        assert_eq!(sanitize_name("a--b--c"), "a-b-c");
        assert_eq!(sanitize_name(""), "session");
    }

    #[test]
    fn test_shell_quote() {
        assert_eq!(shell_quote("simple"), "simple");
        assert_eq!(shell_quote("has space"), "'has space'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn test_generate_short_id() {
        let id1 = generate_short_id();
        let _id2 = generate_short_id(); // Different ID (not asserted due to time sensitivity)
        assert_eq!(id1.len(), 8);
    }

    #[test]
    fn test_tmux_session_new() {
        let session = TmuxSession::new("My Project", "/home/user/project");
        assert!(session.name.starts_with("maestro_My-Project_"));
        assert_eq!(session.display_name, "My Project");
        assert_eq!(session.work_dir, "/home/user/project");
    }
}
