//! Conductor-specific theme and symbols
//!
//! Inspired by Ralph TUI's minimalist and high-density interface.

use ratatui::style::Color;

/// Ralph-inspired theme for the Conductor tab
pub struct ConductorTheme {
    // Backgrounds
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_tertiary: Color,
    pub bg_highlight: Color,

    // Foregrounds
    pub fg_primary: Color,
    pub fg_secondary: Color,
    pub fg_muted: Color,
    pub fg_dim: Color,

    // Status colors
    pub status_success: Color,
    pub status_warning: Color,
    pub status_error: Color,
    pub status_info: Color,

    // Task specific status colors
    pub task_done: Color,
    pub task_active: Color,
    pub task_actionable: Color,
    pub task_pending: Color,
    pub task_blocked: Color,
    pub task_error: Color,
    pub task_closed: Color,

    // Accents
    pub accent_primary: Color,
    pub accent_secondary: Color,
    pub accent_tertiary: Color,
}

impl Default for ConductorTheme {
    fn default() -> Self {
        Self {
            // Tokyo Night inspired palette
            bg_primary: Color::Rgb(26, 27, 38),   // #1a1b26
            bg_secondary: Color::Rgb(36, 40, 59), // #24283b
            bg_tertiary: Color::Rgb(47, 52, 73),  // #2f3449
            bg_highlight: Color::Rgb(61, 66, 89), // #3d4259

            fg_primary: Color::Rgb(192, 202, 245),   // #c0caf5
            fg_secondary: Color::Rgb(169, 177, 214), // #a9b1d6
            fg_muted: Color::Rgb(86, 95, 137),       // #565f89
            fg_dim: Color::Rgb(65, 72, 104),         // #414868

            status_success: Color::Rgb(158, 206, 106), // #9ece6a (green)
            status_warning: Color::Rgb(224, 175, 104), // #e0af68 (yellow)
            status_error: Color::Rgb(247, 118, 142),   // #f7768e (red)
            status_info: Color::Rgb(122, 162, 247),    // #7aa2f7 (blue)

            task_done: Color::Rgb(158, 206, 106),
            task_active: Color::Rgb(158, 206, 106),
            task_actionable: Color::Rgb(158, 206, 106),
            task_pending: Color::Rgb(86, 95, 137),
            task_blocked: Color::Rgb(247, 118, 142),
            task_error: Color::Rgb(247, 118, 142),
            task_closed: Color::Rgb(65, 72, 104),

            accent_primary: Color::Rgb(122, 162, 247), // #7aa2f7
            accent_secondary: Color::Rgb(187, 154, 247), // #bb9af7
            accent_tertiary: Color::Rgb(125, 207, 255), // #7dcfff
        }
    }
}

// Status indicators (Unicode) - Ralph Parity
pub const STATUS_DONE: &str = "✓";
pub const STATUS_ACTIVE: &str = "▶";
pub const STATUS_ACTIONABLE: &str = "○";
pub const STATUS_PENDING: &str = "○";
pub const STATUS_BLOCKED: &str = "⊘";
pub const STATUS_ERROR: &str = "✗";
pub const STATUS_RUNNING: &str = "◐";
pub const STATUS_PAUSED: &str = "⏸";
pub const STATUS_READY: &str = "◉";
