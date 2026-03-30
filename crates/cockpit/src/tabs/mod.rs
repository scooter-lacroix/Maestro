//! Tab rendering functions for Cockpit TUI
//!
//! This module contains all tab-specific rendering functions extracted from app.rs
//! to improve code organization and maintainability.

pub mod analysis;
pub mod capabilities;
pub mod dashboard;
pub mod ktop;
pub mod lsp_registry;
pub mod lsps;
pub mod memory;
pub mod projects;
pub mod sessions;
pub mod settings;
pub mod tracklens;

// Re-export commonly used functions for convenience
pub use analysis::render_analysis;
pub use capabilities::{render_capabilities, CapabilitiesSection};
pub use dashboard::render_dashboard;
pub use ktop::{handle_ktop_input, KtopState};
pub use lsps::render_lsps;
pub use memory::render_memory;
pub use projects::render_projects;
pub use sessions::{render_sessions, session_log_tail};
pub use settings::render_settings;
pub use tracklens::render_tracklens;
