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
pub mod normalized_model;
pub mod omp_agent;
// pub mod mem; // Memory browser overlay
pub mod launch_service;
pub mod memory_browser;
// pub mod mem; // TODO: Implement memory integration module
pub mod conflict_panel;
pub mod keybindings;
pub mod modals;
pub mod model;
pub mod observer;
pub mod pane;
pub mod parallel_view;
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

pub use conflict_panel::*;
pub use dashboard::*;
pub use details_panel::*;
pub use footer::*;
pub use git::*;
pub use header::*;
pub use iteration_history::*;
pub use normalized_model::*;
pub use keybindings::*;
pub use launch_service::*;
pub use modals::{ListSelectorModal, Modal, ModalCancelled, ModalResult, TextInputModal};
pub use model::*;
pub use observer::{
    FileBasedObserver, ObservedSession, ObserverAction, ObserverState, SessionEventBridge,
    SteeringCommand, ToSteeringCommand,
};
pub use pane::*;
pub use parallel_view::*;
pub use project_selector::*;
pub use selector_modal::*;
pub use subagent_tree::*;
pub use theme::*;
pub use track_tree::*;
pub use tree_builder::*;
