//! Modal rendering functions for Cockpit TUI
//!
//! This module contains all modal overlay rendering functions for the Maestro Cockpit TUI.
//! Each modal is a self-contained popup that renders over the main interface.

use ratatui::prelude::*;

/// Helper function to create a centered rectangular area.
///
/// This is used by most modals to position their popup window in the center
/// of the terminal, with configurable width and height percentages.
///
/// # Arguments
/// * `percent_x` - Width percentage (0-100) of the terminal
/// * `percent_y` - Height percentage (0-100) of the terminal
/// * `r` - The parent rectangle (typically `frame.area()`)
///
/// # Returns
/// A `Rect` representing the centered area
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// Sub-modules
pub mod action;
pub mod help;
pub mod mcp;
pub mod overlay;
pub mod sessions;
pub mod settings;
pub mod wizards;

// Re-export commonly used functions for convenience
pub use action::render_action_modal;
pub use help::{build_help_text, render_help_modal};
pub use mcp::{render_mcp_logs_modal, render_mcp_menu};
pub use overlay::render_spawning_overlay;
pub use sessions::{render_session_hub_modal, render_switcher_modal};
pub use settings::render_settings_menu_modal;
pub use wizards::{render_group_modal, render_input_modal, render_new_project_modal, render_new_track_modal};
