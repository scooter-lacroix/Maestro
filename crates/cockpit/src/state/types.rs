//! Cockpit state type definitions
//!
//! This module contains all the enums and structs used for Cockpit TUI state.
//! These are pure data types with no business logic.

use leindex_core::memory::models::{Session, SessionGroup};

/// Input mode for the TUI - determines what input is being captured
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum InputMode {
    Normal,
    NewSessionTitle,
    NewSessionPath,
    NewSessionTool,
    NewMemoryContent,
    NewMemoryCategory,
    SessionSwitcher,
    RenameGroup,
    ForkSession,
    KillConfirm,
    DeleteConfirm,
    AnalysisPrompt,
    MemorySearch,
    MemoryDetail,
    MemoryDetailFocus,
    // Phase 11 additions
    SessionHub,
    NewGroupTitle,
    MoveToGroup,
    McpMenu,
    McpLogs,
    LspInstaller,
    DiagnosticView,
    // Phase 15 additions
    NewProjectName,
    NewProjectPath,
    NewProjectTool,
    NewTrackTitle,
    NewTrackType,
    NewGroupCategory,
    RenameGroupCategory,
    SettingsEditor,
    SettingsInstallPath,
    SettingsMenu,
}

/// Focus areas within the Session Hub
#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub enum HubFocus {
    #[default]
    Rename,
    Group,
    Search,
}

/// MCP menu options
#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub enum McpOption {
    #[default]
    Start,
    Stop,
    Pause,
    Logs,
    Add,
    Install,
    Reinstall,
    Remove,
    Uninstall,
}

impl McpOption {
    pub const ALL: [Self; 9] = [
        Self::Start,
        Self::Stop,
        Self::Pause,
        Self::Logs,
        Self::Add,
        Self::Install,
        Self::Reinstall,
        Self::Remove,
        Self::Uninstall,
    ];

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|option| *option == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        let idx = Self::ALL.iter().position(|option| *option == self).unwrap_or(0);
        if idx == 0 {
            Self::ALL[Self::ALL.len() - 1]
        } else {
            Self::ALL[idx - 1]
        }
    }
}

/// Settings options
#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub enum SettingsOption {
    #[default]
    Editor,
    InstallPath,
    Theme,
    Transparent,
    Save,
}

/// Settings menu kind (for dropdown selection)
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum SettingsMenuKind {
    Editor,
    Theme,
}

/// Dashboard focus areas
#[derive(PartialEq, Eq, Clone, Copy, Default, Debug)]
pub enum DashFocus {
    #[default]
    Sessions,
    Mcp,
    Tabs,
}

/// Session entry (either a group header or a session)
#[derive(Clone)]
pub enum SessionEntry {
    Group(SessionGroup),
    Session(Session),
}

/// Dashboard session entry (group header or session)
#[derive(Clone)]
pub enum DashSessionEntry {
    GroupHeader { group_path: String },
    Session(Session),
}

/// Project information
#[derive(Clone)]
pub struct ProjectInfo {
    pub name: String,
    pub path: String,
    pub _track_count: usize,
}

/// Memory information
#[derive(Clone)]
pub struct MemoryInfo {
    pub id: i64,
    pub content: String,
    pub category: String,
    pub summary: Option<String>,
    pub importance: String,
    pub source: Option<String>,
    pub session_id: Option<String>,
    pub project_id: Option<String>,
    pub track_id: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub last_accessed: Option<String>,
    pub access_count: usize,
    pub accessed_by: Vec<String>,
    pub tags: Vec<String>,
    pub is_expanded: bool,
    pub similarity_score: Option<f32>,
}

/// Dashboard statistics
#[derive(Clone, Default)]
pub struct Stats {
    pub project_count: usize,
    pub memory_count: usize,
    pub track_count: usize,
}

/// Analysis mode for LeIndex 5-phase analysis
#[derive(PartialEq, Eq, Clone, Copy, Default, Debug)]
pub enum AnalysisMode {
    /// Ultra mode - Fast orientation (98% token savings, exploration only)
    #[default]
    Ultra,
    /// Balanced mode - Implementation-ready (82% token savings, LLM actionable)
    Balanced,
}

/// Analysis history entry
#[derive(Clone, Debug)]
pub struct AnalysisHistoryEntry {
    pub timestamp: String,
    pub command: String,
    pub result_summary: String,
    pub mode: AnalysisMode,
}

/// Per-session diagnostic severity counts.
#[derive(Clone, Debug, Default)]
pub struct LspDiagnosticCounts {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
    pub hints: usize,
}

/// Aggregated diagnostics for a single session/LSP grouping.
#[derive(Clone, Debug, Default)]
pub struct LspDiagnosticSummary {
    pub session_id: Option<String>,
    pub session_title: Option<String>,
    pub lsp_name: Option<String>,
    pub counts: LspDiagnosticCounts,
}

/// Top-level LSP status rollup for the cockpit header and tab summaries.
#[derive(Clone, Debug, Default)]
pub struct LspStatusSummary {
    pub total_lsps: usize,
    pub running: usize,
    pub stopped: usize,
    pub errors: usize,
    pub starting: usize,
    pub total_errors: usize,
    pub total_warnings: usize,
}

/// Modal state for the LSP installer chooser.
#[derive(Clone, Debug, Default)]
pub struct LspInstallerState {
    pub is_open: bool,
    pub selected_index: usize,
    pub is_installing: bool,
    pub install_output: Option<String>,
}

/// Selection state for the diagnostic detail modal.
#[derive(Clone, Debug, Default)]
pub struct DiagnosticViewState {
    pub is_open: bool,
    pub selected_index: usize,
}

/// A single LSP diagnostic entry, suitable for rendering and agent handoff.
#[derive(Clone, Debug, Default)]
pub struct LspDiagnosticDetail {
    pub session_id: Option<String>,
    pub session_title: Option<String>,
    pub lsp_name: Option<String>,
    pub file_path: String,
    pub severity: String,
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub source: Option<String>,
    pub code: Option<String>,
}
