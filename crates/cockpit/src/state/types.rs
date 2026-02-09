//! Cockpit state type definitions
//!
//! This module contains all the enums and structs used for Cockpit TUI state.
//! These are pure data types with no business logic.

use leindex_analyzers::memory::models::{Session, SessionGroup};

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
    StartStop,
    Pause,
    Logs,
    Add,
    Remove,
    Install,
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
