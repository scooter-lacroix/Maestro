//! Cockpit state management
//!
//! This module contains all state-related types and structures for the Cockpit TUI.
//! State is organized by concern to avoid circular dependencies.

pub mod types;

// Re-export commonly used types
pub use types::{
    AnalysisHistoryEntry, AnalysisMode, DashFocus, DashSessionEntry, DiagnosticSeverity,
    DiagnosticViewState, HubFocus, InputMode, LspDiagnosticCounts, LspDiagnosticDetail,
    LspDiagnosticSummary, LspInstallerState, LspStatusSummary, MaesterClawSetupCheck, MaesterClawSetupState,
    MaesterClawSetupStep, McpOption, MemoryInfo, ProjectInfo, SessionEntry, SettingsMenuKind,
    SettingsOption, Stats,
};