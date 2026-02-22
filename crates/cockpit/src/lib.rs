//! Maestro Cockpit v2 - Ratatui Terminal UI
//!
//! This crate provides the canonical Maestro TUI with a modular architecture
//! separating UI state, rendering, and actions.

pub mod app;
pub mod command_palette;
pub mod conductor;
pub mod maesterclaw;
pub mod maestro_paths;
pub mod modals;
pub mod omp;
pub mod orchestrate; // Deprecated: use conductor module instead
pub mod state;
pub mod tabs;
pub mod theme;
pub mod toast;
pub mod welcome;

pub use app::run;

/// Re-export commonly used types for convenience
pub use leindex_core::{
    config::Config,
    memory::models::{McpServer, Session},
};
