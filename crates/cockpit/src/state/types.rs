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
    SessionSwitcher,
    RenameGroup,
    ForkSession,
    KillConfirm,
    DeleteConfirm,
    AnalysisPrompt,
    MemorySearch,
    // Phase 11 additions
    SessionHub,
    NewGroupTitle,
    MoveToGroup,
    McpMenu,
    McpLogs,
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
    // LSP Installer
    LspInstaller,
    // Diagnostic Detail View
    DiagnosticView,
    // Memory creation
    NewMemoryContent,
    NewMemoryCategory,
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
    Remove,
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
    pub _id: i64,
    pub content: String,
    pub category: String,
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

/// LSP diagnostic severity counts
#[derive(Clone, Debug, Default)]
pub struct LspDiagnosticCounts {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
    pub hints: usize,
}

/// LSP diagnostic summary for a session
#[derive(Clone, Debug)]
pub struct LspDiagnosticSummary {
    pub session_id: String,
    pub session_title: String,
    pub lsp_name: String,
    pub counts: LspDiagnosticCounts,
    pub last_updated: Option<String>,
}

/// Aggregated LSP status across all sessions
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

/// LSP diagnostic detail for display
#[derive(Clone, Debug)]
pub struct LspDiagnosticDetail {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: Option<String>,
    pub code: Option<String>,
}

/// Diagnostic severity for display
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// LSP installer modal state
#[derive(Clone, Debug, Default)]
pub struct LspInstallerState {
    pub is_open: bool,
    pub selected_index: usize,
    pub install_output: Option<String>,
    pub is_installing: bool,
    pub filter_language: Option<String>,
}

/// Diagnostic detail view state
#[derive(Clone, Debug, Default)]
pub struct DiagnosticViewState {
    pub is_open: bool,
    pub selected_index: usize,
    pub expanded_files: std::collections::HashSet<String>,
    pub show_send_prompt: bool,
}
