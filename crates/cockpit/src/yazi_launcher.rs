//! Yazi File Explorer Launcher
//!
//! Provides integration with the Yazi file explorer, launching it
//! directly in the project directory.

use anyhow::{Context, Result};
use std::process::Command;
use tracing::{debug, info, warn};

/// Launch Yazi file explorer directly
///
/// This function launches Yazi directly in the specified project directory.
/// It bypasses the tab multiplexer to avoid terminal control issues when
/// launching from within a TUI application.
///
/// # Arguments
/// * `project_path` - The project directory path to open in Yazi
/// * `project_name` - The project name (used for logging)
///
/// # Returns
/// Returns Ok(()) when Yazi exits successfully, or an error if something fails.
///
/// # Example
/// ```no_run
/// use maestro_cockpit::yazi_launcher::launch_yazi;
///
/// launch_yazi("/home/user/myproject", "myproject").unwrap();
/// ```
pub fn launch_yazi(project_path: &str, project_name: &str) -> Result<()> {
    // 1. Find Yazi executable
    let yazi_path = find_yazi().ok_or_else(|| {
        anyhow::anyhow!("Yazi not found. Please install Yazi: https://github.com/sxyazi/yazi")
    })?;
    info!("Found Yazi at: {:?}", yazi_path);

    // 2. Expand the project path (handle ~)
    let expanded_path = expand_tilde(project_path);
    debug!("Expanded project path: {}", expanded_path);

    // 3. Verify the directory exists
    if !std::path::Path::new(&expanded_path).exists() {
        return Err(anyhow::anyhow!(
            "Project directory does not exist: {}",
            expanded_path
        ));
    }

    info!("Launching Yazi for project '{}'...", project_name);

    // 4. Build the Yazi command
    // CRITICAL: We must inherit stdin/stdout/stderr for Yazi to have proper terminal control.
    // Capturing any of these streams breaks the TTY connection that Yazi needs.
    // We use .spawn() + .wait() instead of .status() for better control.
    //
    // NOTE: We do NOT call reset_terminal_state() here because:
    // - suspend_fullscreen_app() in app.rs already handles terminal state cleanup
    // - Yazi uses crossterm which will properly initialize the terminal itself
    // - Calling reset_terminal_state() causes competing terminal state manipulation
    let mut cmd = Command::new(&yazi_path);
    cmd.current_dir(&expanded_path)
        .env("YAZI_CONFIG_HOME", get_yazi_config_dir())
        .env(
            "EDITOR",
            std::env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string()),
        )
        .env(
            "TERM",
            std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string()),
        )
        .env(
            "COLORTERM",
            std::env::var("COLORTERM").unwrap_or_else(|_| "truecolor".to_string()),
        );

    // Inherit all stdio handles - this is critical for TUI apps
    cmd.stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    debug!("Spawning Yazi process...");
    let mut child = cmd
        .spawn()
        .context("Failed to launch Yazi. Ensure Yazi is properly installed.")?;

    // Wait for Yazi to complete
    let status = child.wait().context("Failed to wait for Yazi process")?;

    debug!("Yazi process completed with status: {:?}", status);

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        // Yazi returns 130 on Ctrl+C, which is normal exit
        if code == 130 {
            info!("Yazi exited with code 130 (interrupted by user)");
        } else {
            warn!("Yazi exited with code: {}", code);
            return Err(anyhow::anyhow!("Yazi exited with code: {}", code));
        }
    }

    info!("Yazi session completed successfully");

    Ok(())
}

/// Check if Yazi is available in the system
pub fn is_yazi_available() -> bool {
    find_yazi().is_some()
}

/// Get a status report of Yazi launcher dependencies
pub fn get_status_report() -> YaziLauncherStatus {
    YaziLauncherStatus {
        yazi_available: is_yazi_available(),
        yazi_path: find_yazi(),
    }
}

/// Status report for Yazi launcher dependencies
#[derive(Debug, Clone)]
pub struct YaziLauncherStatus {
    pub yazi_available: bool,
    pub yazi_path: Option<std::path::PathBuf>,
}

impl YaziLauncherStatus {
    /// Check if Yazi is available
    pub fn is_ready(&self) -> bool {
        self.yazi_available
    }

    /// Get a human-readable status message
    pub fn status_message(&self) -> String {
        if self.is_ready() {
            "Yazi launcher ready".to_string()
        } else if !self.yazi_available {
            "Yazi not found - install from https://github.com/sxyazi/yazi".to_string()
        } else {
            "Unknown dependency issue".to_string()
        }
    }
}

// ============================================================================
// Internal Helper Functions
// ============================================================================

/// Find the Yazi executable using maestro_core's portability module
fn find_yazi() -> Option<std::path::PathBuf> {
    maestro_core::portability::executable::find_yazi()
}

/// Expand tilde (~) in a path string
fn expand_tilde(path: &str) -> String {
    if path == "~" {
        return dirs::home_dir()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
    }

    if let Some(rest) = path.strip_prefix("~/") {
        return dirs::home_dir()
            .map(|h| h.join(rest).to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
    }

    path.to_string()
}
/// Get the Yazi configuration directory
fn get_yazi_config_dir() -> String {
    // Check YAZI_CONFIG_HOME first
    if let Ok(config_home) = std::env::var("YAZI_CONFIG_HOME") {
        return config_home;
    }

    // Fall back to XDG config
    if let Some(config_dir) = dirs::config_dir() {
        return config_dir.join("yazi").to_string_lossy().to_string();
    }

    // Final fallback
    if let Some(home) = dirs::home_dir() {
        return home.join(".config/yazi").to_string_lossy().to_string();
    }

    "/tmp".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde() {
        let expanded = expand_tilde("~");
        assert!(!expanded.starts_with('~'));

        let expanded = expand_tilde("~/test/path");
        assert!(!expanded.starts_with('~'));
        assert!(expanded.ends_with("test/path"));

        let no_tilde = expand_tilde("/usr/bin/test");
        assert_eq!(no_tilde, "/usr/bin/test");
    }

    #[test]
    fn test_is_yazi_available() {
        // This test just verifies the function doesn't panic
        let _available = is_yazi_available();
    }

    #[test]
    fn test_status_report() {
        let status = get_status_report();
        // Just verify it compiles and runs
        let _ = status.is_ready();
        let _ = status.status_message();
    }
}
