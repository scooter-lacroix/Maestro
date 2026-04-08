//! Maestro Cockpit v2 - Ratatui Terminal UI
//!
//! This crate provides the canonical Maestro TUI with a modular architecture
//! separating UI state, rendering, and actions.

pub mod app;
pub mod cockpit_log;
pub mod command_palette;
pub mod conductor;
pub mod maesterclaw; // Legacy module for backward compatibility
pub mod maestro_paths;
pub mod modals;
pub mod omp;
pub mod orchestrate; // Deprecated: use conductor module instead
pub mod state;
pub mod tabs;
pub mod theme;
pub mod toast;
pub mod tracklens;
pub mod welcome;
pub mod yazi_launcher;

pub use app::run;

/// Re-export commonly used types for convenience
pub use leindex_core::{
    config::Config,
    memory::models::{McpServer, Session},
};
