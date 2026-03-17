//! Conductor module
//!
//! Port of Ralph TUI functionality into Maestro Cockpit.

pub mod agent_executor;
pub mod dashboard;
pub mod details_panel;
pub mod footer;
pub mod git;
pub mod header;
pub mod input_modal;
pub mod iteration_history;
pub mod keybindings;
pub mod launch_service;
pub mod memory_browser;
pub mod model;
pub mod modals;
pub mod normalized_model;
pub mod omp_agent;
pub mod pane;
pub mod polling;
pub mod project_selector;
pub mod selector_modal;
pub mod state_machine;
pub mod subagent_tree;
pub mod telemetry;
#[cfg(test)]
pub mod tests;
pub mod theme;
pub mod track_tree;
pub mod tree_builder;

// Re-exports from each module
pub use agent_executor::*;
pub use dashboard::*;
pub use details_panel::*;
pub use footer::*;
pub use git::*;
pub use header::*;
pub use iteration_history::*;
pub use keybindings::*;
pub use launch_service::*;
pub use model::*;
pub use modals::{ListSelectorModal, Modal, ModalCancelled, ModalResult, TextInputModal};
pub use normalized_model::*;
pub use pane::*;
pub use project_selector::*;
pub use selector_modal::*;
pub use subagent_tree::*;
pub use theme::*;
pub use track_tree::*;
