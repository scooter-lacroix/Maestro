//! Tab rendering functions for Cockpit TUI
//!
//! This module contains all tab-specific rendering functions extracted from app.rs
//! to improve code organization and maintainability.

pub mod dashboard;
pub mod memory;
pub mod analysis;
pub mod settings;
pub mod lsps;
pub mod projects;
pub mod sessions;

// Re-export commonly used functions for convenience
pub use dashboard::render_dashboard;
pub use memory::render_memory;
pub use analysis::render_analysis;
pub use settings::render_settings;
pub use lsps::{render_lsps, get_lsp_install_command};
pub use projects::render_projects;
pub use sessions::{render_sessions, session_log_tail};
