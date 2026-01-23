//! Zellij Multiplexer Integration
//!
//! Provides high-performance terminal multiplexing using Zellij.

use anyhow::{Context, Result};
use std::process::Command;
use tracing::{debug, info};

pub struct ZellijMultiplexer;

impl ZellijMultiplexer {
    fn expand_tilde(path: &str) -> String {
        if path == "~" || path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                if path == "~" {
                    return home.to_string_lossy().to_string();
                }
                if let Some(rest) = path.strip_prefix("~/") {
                    return home.join(rest).to_string_lossy().to_string();
                }
            }
        }
        path.to_string()
    }

    /// Check if we are currently running inside a Zellij session
    pub fn is_in_zellij() -> bool {
        std::env::var("ZELLIJ").is_ok()
    }

    /// Launch Zide for a project
    pub fn spawn_zide(project_path: &str, name: &str) -> Result<()> {
        let project_path = Self::expand_tilde(project_path);

        // Find bundled resources
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        let user_zide_path = home.join(".maestro").join("resources").join("zide");
        let user_zide_str = user_zide_path.to_string_lossy();

        let possible_paths = [
            "/home/stan/Prod/maestro/maestro/leindex/rust/resources/zide",
            "/home/stan/Prod/maestro/leindex/rust/resources/zide",
            &user_zide_str,
        ];

        let zide_dir = possible_paths
            .iter()
            .find(|p| std::path::Path::new(p).exists())
            .map(|p| p.to_string())
            .unwrap_or_else(|| "/usr/local/share/maestro/zide".to_string());

        let zide_bin = format!("{}/bin/zide", zide_dir);

        info!(
            "Launching bundled Zide for {} at {} (resources: {})",
            name, project_path, zide_dir
        );

        let mut cmd = Command::new(&zide_bin);
        cmd.arg("-n").arg(name);
        cmd.arg(project_path);
        cmd.arg("maestro_yazi.kdl"); // Explicitly add .kdl

        // Ensure environment is primed for Zide
        cmd.env("ZIDE_DIR", &zide_dir);

        // Add Zide bin to PATH so it can find zide-pick, zide-edit etc.
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/home/stan"));
        let local_bin = home.join(".local").join("bin");
        let cargo_bin = home.join(".cargo").join("bin");
        let current_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!(
            "{}/bin:{}:{}:{}:{}",
            zide_dir,
            local_bin.to_string_lossy(),
            cargo_bin.to_string_lossy(),
            "/usr/local/bin",
            current_path
        );
        cmd.env("PATH", new_path);

        // Crucial: Fix the missing EDITOR error by ensuring it's set
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
            info!("EDITOR not set, defaulting to 'fresh' for Maestro");
            "fresh".to_string()
        });
        cmd.env("EDITOR", &editor);
        cmd.env("ZIDE_ORIGINAL_EDITOR", &editor);

        info!("Zide using editor: {} (from EDITOR env or default)", editor);

        // Run blocking so TUI pauses and Zide takes over the terminal
        cmd.status()
            .context("Failed to run Zide. Ensure path is correct and executable.")?;

        Ok(())
    }

    /// Create a new Zellij session in the background or ensure it exists
    pub fn ensure_session(name: &str) -> Result<()> {
        debug!("Ensuring Zellij session: {}", name);

        // Use zellij attach -b to create in background if it doesn't exist
        let status = Command::new("zellij")
            .arg("attach")
            .arg("-b")
            .arg(name)
            .status()
            .context("Failed to ensure Zellij session exists (attach -b)")?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "Zellij failed to create/attach session background: {}",
                name
            ));
        }

        // Give Zellij a moment to initialize the session socket
        std::thread::sleep(std::time::Duration::from_millis(300));

        // Optional: verify it exists in list-sessions
        let mut retries = 5;
        while retries > 0 {
            let output = Command::new("zellij").arg("list-sessions").output()?;
            let sessions = String::from_utf8_lossy(&output.stdout);
            if sessions.contains(name) {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
            retries -= 1;
        }

        Err(anyhow::anyhow!(
            "Session {} failed to appear in list-sessions after creation",
            name
        ))
    }

    /// Run a command in a new Zellij pane
    pub fn run_in_pane(session_name: &str, command: &str, cwd: Option<&str>) -> Result<()> {
        debug!("Running command in Zellij pane: {}", command);

        // Find bundled resources
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        let user_zide_path = home.join(".maestro").join("resources").join("zide");
        let user_zide_str = user_zide_path.to_string_lossy();

        let possible_paths = [
            "/home/stan/Prod/maestro/maestro/leindex/rust/resources/zide",
            "/home/stan/Prod/maestro/leindex/rust/resources/zide",
            &user_zide_str,
        ];

        let zide_dir = possible_paths
            .iter()
            .find(|p| std::path::Path::new(p).exists())
            .map(|p| p.to_string())
            .unwrap_or_else(|| "/usr/local/share/maestro/zide".to_string());

        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/home/stan"));
        let local_bin = home.join(".local").join("bin");
        let cargo_bin = home.join(".cargo").join("bin");
        let current_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!(
            "{}/bin:{}:{}:{}:{}",
            zide_dir,
            local_bin.to_string_lossy(),
            cargo_bin.to_string_lossy(),
            "/usr/local/bin",
            current_path
        );
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

        let mut cmd = Command::new("zellij");
        cmd.arg("-s")
            .arg(session_name)
            .arg("action")
            .arg("new-pane");

        if let Some(dir) = cwd {
            cmd.arg("--cwd").arg(dir);
        }

        cmd.env("PATH", new_path);
        cmd.env("EDITOR", editor);
        cmd.env("ZIDE_DIR", zide_dir);

        cmd.arg("--").arg("bash").arg("-c").arg(command);

        cmd.status().context("Failed to spawn Zellij pane")?;

        Ok(())
    }

    /// Switch to or attach to a Zellij session
    pub fn switch_session(name: &str) -> Result<()> {
        if Self::is_in_zellij() {
            info!("Switching to Zellij session: {}", name);
            Command::new("zellij")
                .arg("action")
                .arg("switch-session")
                .arg(name)
                .status()
                .context("Failed to switch Zellij session")?;
        } else {
            Self::attach(name)?;
        }
        Ok(())
    }

    /// Focus/Attach to a Zellij session
    pub fn attach(session_name: &str) -> Result<()> {
        let mut cmd = Command::new("zellij");
        cmd.arg("attach").arg(session_name);

        cmd.status().context("Failed to attach to Zellij session")?;

        Ok(())
    }
}
