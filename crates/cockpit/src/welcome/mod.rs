//! Welcome screen and onboarding wizard for Maestro Cockpit
//!
//! This module provides first-time user onboarding with a step-by-step wizard
//! following MaesterClaw design principles.

mod screen;
#[cfg(test)]
mod tests;
mod wizard;

pub use screen::WelcomeScreen;
pub use wizard::{WelcomeState, WelcomeStep};

use std::path::PathBuf;

/// Marker file path for first-time detection
pub fn cockpit_initialized_marker() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    PathBuf::from(home)
        .join(".maestro")
        .join(".cockpit_initialized")
}

/// Check if this is a first-time user
pub fn is_first_time_user() -> bool {
    !cockpit_initialized_marker().exists()
}

/// Mark the cockpit as initialized (after onboarding completion)
pub fn mark_initialized() -> std::io::Result<()> {
    let marker = cockpit_initialized_marker();
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&marker, chrono::Utc::now().to_rfc3339())
}

/// Default workspace path
pub fn default_workspace_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    format!("{}/.maestro/workspace", home)
}

/// Available editors for selection
pub const AVAILABLE_EDITORS: &[(&str, &str)] = &[
    ("helix", "Helix editor (hx)"),
    ("neovim", "Neovim (nvim)"),
    ("vim", "Vim"),
    ("vscode", "Visual Studio Code (code)"),
    ("zed", "Zed editor"),
    ("custom", "Custom editor command"),
];

/// Available themes
pub const AVAILABLE_THEMES: &[(&str, &str)] = &[
    ("system", "System (respects terminal transparency)"),
    ("dark", "Dark theme"),
    ("light", "Light theme"),
    ("dracula", "Dracula theme"),
    ("nord", "Nord theme"),
];
