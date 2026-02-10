//! Tab rendering functions for Cockpit TUI
//!
//! This module contains all tab-specific rendering functions extracted from app.rs
//! to improve code organization and maintainability.

pub mod analysis;
pub mod dashboard;
pub mod lsps;
pub mod memory;
pub mod projects;
pub mod sessions;
pub mod settings;

// Re-export commonly used functions for convenience
pub use analysis::render_analysis;
pub use dashboard::render_dashboard;
pub use lsps::{get_lsp_install_command, render_lsps};
pub use memory::render_memory;
pub use projects::render_projects;
pub use sessions::{render_sessions, session_log_tail};
pub use settings::render_settings;
