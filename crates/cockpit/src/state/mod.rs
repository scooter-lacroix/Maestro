//! Cockpit state management
//!
//! This module contains all state-related types and structures for the Cockpit TUI.
//! State is organized by concern to avoid circular dependencies.

pub mod types;

// Re-export commonly used types
pub use types::{
<<<<<<< HEAD
    AnalysisMode, AnalysisHistoryEntry, DashFocus, DashSessionEntry, HubFocus, InputMode,
    McpOption, MemoryInfo, ProjectInfo, SessionEntry, SettingsMenuKind, SettingsOption, Stats,
=======
    DashFocus, DashSessionEntry, HubFocus, InputMode, McpOption, MemoryInfo, ProjectInfo,
    SessionEntry, SettingsMenuKind, SettingsOption, Stats,
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)
};
