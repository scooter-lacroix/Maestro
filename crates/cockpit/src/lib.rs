//! Maestro Cockpit v2 - Ratatui Terminal UI
//!
//! This crate provides the canonical Maestro TUI with a modular architecture
//! separating UI state, rendering, and actions.

pub mod app;
pub mod modals;
pub mod orchestrate;
pub mod state;
pub mod tabs;
pub mod theme;

pub use app::run;

/// Re-export commonly used types for convenience
pub use leindex_core::{
    memory::models::{Session, McpServer},
    config::Config,
};
