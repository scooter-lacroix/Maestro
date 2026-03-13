//! Cockpit state management
//!
//! This module contains all state-related types and structures for the Cockpit TUI.
//! State is organized by concern to avoid circular dependencies.

pub mod types;

// Re-export commonly used types
pub use types::{
    AnalysisHistoryEntry, AnalysisMode, DashFocus, DashSessionEntry, DiagnosticViewState, HubFocus,
    InputMode, LspDiagnosticCounts, LspDiagnosticDetail, LspDiagnosticSummary, LspInstallerState,
    LspStatusSummary, McpOption, MemoryInfo, ProjectInfo, SessionEntry, SettingsMenuKind,
    SettingsOption, Stats,
};
