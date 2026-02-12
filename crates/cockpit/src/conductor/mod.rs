//! Conductor module
//!
//! Port of Ralph TUI functionality into Maestro Cockpit.

pub mod omp_agent;
pub mod dashboard;
pub mod details_panel;
pub mod footer;
pub mod git;
pub mod header;
pub mod iteration_history;
pub mod keybindings;
pub mod model;
pub mod pane;
pub mod polling;
pub mod project_selector;
pub mod state_machine;
pub mod subagent_tree;
pub mod telemetry;
#[cfg(test)]
pub mod tests;
pub mod theme;
pub mod track_tree;

pub use dashboard::*;
pub use details_panel::*;
pub use footer::*;
pub use git::*;
pub use header::*;
pub use iteration_history::*;
pub use keybindings::*;
pub use model::*;
pub use pane::*;
pub use project_selector::*;
pub use subagent_tree::*;
pub use theme::*;
pub use track_tree::*;
