//! TUI command implementation
//!
//! Beautiful Terminal User Interface using ratatui.
//! Shows projects, memories, and analysis status.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Paragraph, Tabs},
};
use std::hash::{Hash, Hasher};
use std::{
    collections::hash_map::DefaultHasher,
    collections::{HashMap, HashSet},
    io,
    sync::Arc,
    time::Instant,
};
use tracing::debug;

use leindex_core::config::Config;
use leindex_core::memory::lsp_manager::LspType;
use leindex_core::memory::LspStatus;
use leindex_core::memory::McpPool;
use leindex_core::memory::MemoryService;
use leindex_core::memory::TursoStorageBackend;
use leindex_core::multiplexer::TmuxMultiplexer;

use crate::modals;
use crate::conductor::omp_agent::OmpAgentManager;
use crate::omp::{is_omp_available, OmpWorkerStatus};
use crate::state::{
    AnalysisMode, DashFocus, DashSessionEntry, HubFocus, InputMode, McpOption, MemoryInfo,
    ProjectInfo, SessionEntry, SettingsMenuKind, SettingsOption, Stats,
};
use crate::tabs::{
    render_analysis, render_dashboard, render_lsps, render_memory, render_projects,
    render_sessions, render_settings, session_log_tail,
};

// Re-export for use in tabs
pub use crate::tabs::ktop::{render_ktop, KtopState};
use crate::theme::{theme_from_name, Theme, THEMES};

pub async fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create TursoStorageBackend for LSP operations (FIRST, before any other DB access)
    // This ensures libSQL can configure SQLite threading before it's initialized by rusqlite.
    let storage_backend = match TursoStorageBackend::new(None, None).await {
        Ok(backend) => {
            if let Err(e) = backend.initialize().await {
                eprintln!("Warning: Failed to initialize LSP storage backend: {}", e);
            }
            Some(Arc::new(backend))
        }
        Err(e) => {
            eprintln!(
                "Warning: Failed to create storage backend for LSP operations: {}",
                e
            );
            None
        }
    };

    // Initialize service for live data + system-wide integration (MCP + Memory)
    let service = MemoryService::new(None).ok();
    let mcp_pool: Option<Arc<McpPool>> = if let Some(ref s) = service {
        let _ = s.initialize();
        let _ = s.sync_mcp_servers_from_system();
        let _ = s.sync_memories_from_system();

        // Start pooled MCP servers in the background so all tools can share them.
        let pool = Arc::new(McpPool::new(s.clone()));
        let pool_bg = pool.clone();
        tokio::spawn(async move {
            let _ = pool_bg.start_all_from_db().await;
        });
        Some(pool)
    } else {
        None
    };

    // Run app
    let result = run_app(&mut terminal, service, mcp_pool, storage_backend).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

pub struct App {
    pub tab_index: usize,
    pub should_quit: bool,
    pub show_help: bool,
    pub input_mode: InputMode,
    pub projects: Vec<ProjectInfo>,
    pub project_state: ratatui::widgets::ListState,
    pub memories: Vec<MemoryInfo>,
    pub memory_state: ratatui::widgets::ListState,
    pub memory_query: String,
    pub new_memory_content: String,
    pub new_memory_category: String,
    pub sessions: Vec<leindex_core::memory::models::Session>,
    pub session_entries: Vec<SessionEntry>,
    pub session_state: ratatui::widgets::ListState,
    pub groups: Vec<leindex_core::memory::models::SessionGroup>,
    pub mcp_servers: Vec<leindex_core::memory::models::McpServer>,
    pub stats: Stats,
    pub scroll: usize,
    // Session switcher state
    pub switcher_state: ratatui::widgets::ListState,
    // Input fields for new session
    pub new_session_title: String,
    pub new_session_path: String,
    pub new_session_tool: String,
    pub rename_buffer: String,
    pub target_session_id: Option<String>,
    pub target_group_path: Option<String>,
    // Status & Feedback
    pub is_spawning: bool,
    pub status_message: String,
    pub toast_queue: crate::toast::ToastQueue,
    pub session_preview_content: String,
    // Analysis Hub state
    pub analysis_input: String,
    pub analysis_history: Vec<String>,
    pub analysis_mode: AnalysisMode,
    pub frame_count: u64,
    // Help modal state
    pub help_scroll: u16,
    // Throttle expensive preview capture
    pub last_preview_refresh: Instant,
    // Phase 11 additions
    pub mcp_state: ratatui::widgets::ListState,
    pub preview_focused: bool,
    pub preview_scroll: u16,
    pub hub_search_buffer: String,
    pub hub_focus: HubFocus,
    // Dashboard MCP menu state
    pub mcp_menu_option: McpOption,
    pub target_mcp_name: Option<String>,
    pub mcp_pool: Option<Arc<McpPool>>,
    pub mcp_log_lines: Vec<String>,
    pub mcp_log_scroll: u16,
    // Projects tab state
    pub project_view_open: bool,
    // Phase 15 state
    pub new_project_name: String,
    pub new_project_path: String,
    pub new_project_tool: String,
    pub new_track_title: String,
    pub new_track_is_master: bool,
    pub new_group_category: String,
    // Project Explorer state
    pub project_explorer_path: Option<String>,
    pub project_explorer_selected: usize,
    pub explorer_items: Vec<String>,
    pub config: Config,
    pub settings_option: SettingsOption,
    pub settings_menu_kind: Option<SettingsMenuKind>,
    pub settings_menu_state: ratatui::widgets::ListState,
    pub settings_menu_items: Vec<(String, String)>,
    pub dash_session_state: ratatui::widgets::ListState,
    pub dash_session_entries: Vec<DashSessionEntry>,
    pub dash_focus: DashFocus,
    // Phase 6: LSP Integration
    // Cache of (session_id -> Vec<(lsp_name, status)>)
    pub lsp_status_cache: HashMap<String, Vec<(String, LspStatus)>>,
    pub last_lsp_refresh: Instant,
    pub lsp_state: ratatui::widgets::ListState,
    // LSP log viewing
    pub lsp_log_content: String,
    pub lsp_log_scroll: u16,
    pub lsp_log_source: Option<(String, String)>, // (session_id, lsp_name)
    // LSP installation guidance - tracks which LSPs are available on the system
    pub lsp_availability: HashMap<String, bool>, // lsp_name -> is_available
    // LSP diagnostic summaries for all sessions
    pub lsp_diagnostic_summaries: Vec<crate::state::LspDiagnosticSummary>,
    // LSP aggregated status summary
    pub lsp_status_summary: crate::state::LspStatusSummary,
    // Detected LSPs per session (regardless of running state)
    pub lsp_detected_cache: HashMap<String, Vec<String>>, // session_id -> [lsp_name, ...]
    // LSP installer modal state
    pub lsp_installer: crate::state::LspInstallerState,
    // Diagnostic detail view state
    pub diagnostic_view: crate::state::DiagnosticViewState,
    // Cached diagnostic details from LSP
    pub lsp_diagnostics_cache: Vec<crate::state::LspDiagnosticDetail>,
    // Storage backend for LSP operations (sync access)
    pub storage_backend: Option<Arc<TursoStorageBackend>>,
    // Flag to trigger async LSP refresh
    pub pending_lsp_refresh: bool,
    // LSP Pool for shared LSP instances (preferred)
    pub lsp_pool: Option<Arc<leindex_core::memory::lsp_pool::LspPool>>,
    // Persistent LSP Manager to keep processes alive (legacy, per-session)
    pub lsp_manager: Option<leindex_core::memory::lsp_manager::LspManager>,
    // Sessions that already ran LSP auto-detection
    pub lsp_autostarted_sessions: HashSet<String>,
    // Conductor pane state (formerly Orchestrate)
    pub conductor: crate::conductor::ConductorPane,
    // MCP status refresh task
    mcp_refresh_task: Option<tokio::task::JoinHandle<()>>,
    // Ktop tab state (system resource monitoring)
    pub ktop_state: Option<crate::tabs::ktop::KtopState>,
 // OMP agent manager for tool execution
 pub omp_manager: Option<OmpAgentManager>,
}

// Note: Type definitions (InputMode, HubFocus, McpOption, SettingsOption, SettingsMenuKind,
// DashFocus, SessionEntry, DashSessionEntry, ProjectInfo, MemoryInfo, Stats) are now
// imported from crate::state module to avoid duplication.

impl App {
    fn new(
        _service: Option<&MemoryService>,
        mcp_pool: Option<Arc<McpPool>>,
        storage_backend: Option<Arc<TursoStorageBackend>>,
    ) -> Self {
        let config = Config::load();
        std::env::set_var("EDITOR", &config.editor);
        let mut app = Self {
            tab_index: 0,
            should_quit: false,
            show_help: false,
            input_mode: InputMode::Normal,
            projects: Vec::new(),
            project_state: ratatui::widgets::ListState::default(),
            memories: Vec::new(),
            memory_state: ratatui::widgets::ListState::default(),
            memory_query: String::new(),
            new_memory_content: String::new(),
            new_memory_category: String::new(),
            sessions: Vec::new(),
            session_entries: Vec::new(),
            session_state: ratatui::widgets::ListState::default(),
            groups: Vec::new(),
            mcp_servers: Vec::new(),
            stats: Stats::default(),
            scroll: 0,
            switcher_state: ratatui::widgets::ListState::default(),
            new_session_title: String::new(),
            new_session_path: String::new(),
            new_session_tool: "claude".to_string(),
            rename_buffer: String::new(),
            target_session_id: None,
            target_group_path: None,
            is_spawning: false,
            status_message: String::new(),
            toast_queue: crate::toast::ToastQueue::new(),
            session_preview_content: String::new(),
            analysis_input: String::new(),
            analysis_history: Vec::new(),
            analysis_mode: AnalysisMode::Ultra,
            frame_count: 0,
            help_scroll: 0,
            last_preview_refresh: Instant::now(),
            mcp_state: ratatui::widgets::ListState::default(),
            preview_focused: false,
            preview_scroll: 0,
            hub_search_buffer: String::new(),
            hub_focus: HubFocus::Rename,
            mcp_menu_option: McpOption::Start,
            target_mcp_name: None,
            mcp_pool,
            mcp_log_lines: Vec::new(),
            mcp_log_scroll: 0,
            project_view_open: false,
            new_project_name: String::new(),
            new_project_path: String::new(),
            new_project_tool: String::new(),
            new_track_title: String::new(),
            new_track_is_master: true,
            new_group_category: String::new(),
            project_explorer_path: None,
            project_explorer_selected: 0,

            explorer_items: Vec::new(),
            config,
            settings_option: SettingsOption::Editor,
            settings_menu_kind: None,
            settings_menu_state: ratatui::widgets::ListState::default(),
            settings_menu_items: Vec::new(),
            dash_session_state: ratatui::widgets::ListState::default(),
            dash_session_entries: Vec::new(),
            dash_focus: DashFocus::Sessions,
            lsp_status_cache: HashMap::new(),
            last_lsp_refresh: Instant::now(),
            lsp_state: ratatui::widgets::ListState::default(),
            lsp_log_content: String::new(),
            lsp_log_scroll: 0,
            lsp_log_source: None,
            lsp_availability: HashMap::new(),
            lsp_diagnostic_summaries: Vec::new(),
            lsp_status_summary: crate::state::LspStatusSummary::default(),
            lsp_detected_cache: HashMap::new(),
            lsp_installer: crate::state::LspInstallerState::default(),
            diagnostic_view: crate::state::DiagnosticViewState::default(),
            lsp_diagnostics_cache: Vec::new(),
            storage_backend: storage_backend.clone(),
            pending_lsp_refresh: false,
            lsp_pool: storage_backend.as_ref().map(|s| {
                Arc::new(leindex_core::memory::lsp_pool::LspPool::new(
                    (**s).clone(),
                    leindex_core::memory::lsp_pool::LspPoolConfig::default(),
                ))
            }),
            lsp_manager: storage_backend
                .map(|s| leindex_core::memory::lsp_manager::LspManager::new((*s).clone())),
            lsp_autostarted_sessions: HashSet::new(),
            conductor: crate::conductor::ConductorPane::auto_discover(),
            mcp_refresh_task: None,
            ktop_state: None,
 omp_manager: if is_omp_available() { Some(OmpAgentManager::new(None)) } else { None },
        };

        // Start MCP status refresh background task
        if let Some(ref mcp_pool) = app.mcp_pool {
            let pool = mcp_pool.clone();
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            app.mcp_refresh_task = Some(tokio::spawn(async move {
                loop {
                    interval.tick().await;
                    if let Err(e) = pool.refresh_all_statuses().await {
                        eprintln!("Failed to refresh MCP statuses: {}", e);
                    }
                }
            }));
        }

        // Restore running LSPs (if any were running in a previous instance or should be running)
        if let Some(ref lsp_manager) = app.lsp_manager {
            let manager = lsp_manager.clone();
            tokio::spawn(async move {
                if let Err(e) = manager.restore_lsps_on_startup().await {
                    eprintln!("Failed to restore LSPs: {}", e);
                }
            });
        }

        // Start LSP pool idle monitor
        if let Some(ref lsp_pool) = app.lsp_pool {
            lsp_pool.start_monitor();
        }

        // Check LSP availability on startup
        app.check_lsp_availability();
        app.mcp_state.select(Some(0));
        app.dash_session_state.select(Some(0));
        app.memory_state.select(Some(0));
        app.lsp_state.select(Some(0));
        app
    }

    pub fn theme(&self) -> Theme {
        let mut theme = theme_from_name(&self.config.theme);
        if self.config.transparent {
            theme.bg = Color::Reset; // Use Reset for true transparency
            theme.panel_bg = Color::Reset;
            theme.transparent = true;
        }
        theme
    }

    fn open_settings_menu(&mut self, kind: SettingsMenuKind) {
        self.settings_menu_kind = Some(kind);
        self.settings_menu_items = match kind {
            SettingsMenuKind::Editor => vec![
                ("hx".to_string(), "Helix (hx)".to_string()),
                ("nvim".to_string(), "Neovim (nvim)".to_string()),
                ("vim".to_string(), "Vim (vim)".to_string()),
                ("code".to_string(), "VS Code (code)".to_string()),
                ("zed".to_string(), "Zed (zed)".to_string()),
                ("fresh".to_string(), "Fresh (fresh)".to_string()),
                ("custom".to_string(), "Custom...".to_string()),
            ],
            SettingsMenuKind::Theme => THEMES
                .iter()
                .map(|(id, label)| (id.to_string(), label.to_string()))
                .collect(),
        };

        let current = match kind {
            SettingsMenuKind::Editor => self.config.editor.as_str(),
            SettingsMenuKind::Theme => self.config.theme.as_str(),
        }
        .trim()
        .to_lowercase();

        let selected = self
            .settings_menu_items
            .iter()
            .position(|(id, _)| id.to_lowercase() == current)
            .unwrap_or(0);
        self.settings_menu_state.select(Some(selected));
        self.input_mode = InputMode::SettingsMenu;
    }

    fn dash_selected_session_id(&self) -> Option<String> {
        let selected = self.dash_session_state.selected()?;
        self.dash_session_entries
            .get(selected)
            .and_then(|entry| match entry {
                DashSessionEntry::Session(s) => Some(s.session_id.clone()),
                DashSessionEntry::GroupHeader { .. } => self
                    .dash_session_entries
                    .iter()
                    .skip(selected + 1)
                    .find_map(|e| match e {
                        DashSessionEntry::Session(s) => Some(s.session_id.clone()),
                        DashSessionEntry::GroupHeader { .. } => None,
                    }),
            })
    }

    fn dash_selected_session(&self) -> Option<&leindex_core::memory::models::Session> {
        let selected = self.dash_session_state.selected()?;
        match self.dash_session_entries.get(selected) {
            Some(DashSessionEntry::Session(s)) => Some(s),
            Some(DashSessionEntry::GroupHeader { .. }) => self
                .dash_session_entries
                .iter()
                .skip(selected + 1)
                .find_map(|e| match e {
                    DashSessionEntry::Session(s) => Some(s),
                    DashSessionEntry::GroupHeader { .. } => None,
                }),
            None => None,
        }
    }

    fn dash_first_session_index(&self) -> Option<usize> {
        self.dash_session_entries
            .iter()
            .position(|e| matches!(e, DashSessionEntry::Session(_)))
    }

    fn dash_select_first_session(&mut self) {
        if let Some(idx) = self.dash_first_session_index() {
            self.dash_session_state.select(Some(idx));
        } else {
            self.dash_session_state.select(Some(0));
        }
    }

    fn dash_select_next_session(&mut self) {
        let len = self.dash_session_entries.len();
        if len == 0 {
            self.dash_session_state.select(Some(0));
            return;
        }

        let start = self.dash_session_state.selected().unwrap_or(0);
        let mut idx = start;
        for _ in 0..len {
            idx = if idx >= len.saturating_sub(1) {
                0
            } else {
                idx + 1
            };
            if matches!(
                self.dash_session_entries.get(idx),
                Some(DashSessionEntry::Session(_))
            ) {
                self.dash_session_state.select(Some(idx));
                return;
            }
        }
    }

    fn dash_select_prev_session(&mut self) {
        let len = self.dash_session_entries.len();
        if len == 0 {
            self.dash_session_state.select(Some(0));
            return;
        }

        let start = self.dash_session_state.selected().unwrap_or(0);
        let mut idx = start;
        for _ in 0..len {
            idx = if idx == 0 {
                len.saturating_sub(1)
            } else {
                idx - 1
            };
            if matches!(
                self.dash_session_entries.get(idx),
                Some(DashSessionEntry::Session(_))
            ) {
                self.dash_session_state.select(Some(idx));
                return;
            }
        }
    }

    // LSP installation guidance helpers
    fn check_lsp_availability(&mut self) {
        let lsps = vec!["rust-analyzer", "ruff", "typescript-language-server"];

        for lsp in lsps {
            let available = Self::binary_exists(lsp);
            self.lsp_availability.insert(lsp.to_string(), available);
        }
    }

    fn binary_exists(name: &str) -> bool {
        // Platform-specific binary detection
        #[cfg(target_os = "windows")]
        {
            // On Windows, use 'where' command
            if let Ok(output) = std::process::Command::new("where").arg(name).output() {
                if output.status.success() {
                    return true;
                }
            }

            // Special case for rust-analyzer: check rustup components
            if name == "rust-analyzer" {
                if let Ok(output) = std::process::Command::new("rustup")
                    .args(["component", "list", "--installed"])
                    .output()
                {
                    if let Ok(stdout) = String::from_utf8(output.stdout) {
                        return stdout.contains("rust-analyzer");
                    }
                }
            }

            false
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On Unix-like systems, use 'which' command
            if let Ok(output) = std::process::Command::new("which").arg(name).output() {
                if output.status.success() {
                    return true;
                }
            }

            // Special case for rust-analyzer: check rustup components
            if name == "rust-analyzer" {
                if let Ok(output) = std::process::Command::new("rustup")
                    .args(["component", "list", "--installed"])
                    .output()
                {
                    if let Ok(stdout) = String::from_utf8(output.stdout) {
                        if stdout.contains("rust-analyzer") {
                            return true;
                        }
                    }
                }

                // Also check if rust-analyzer binary exists directly
                if let Ok(output) = std::process::Command::new("which")
                    .arg("rust-analyzer")
                    .output()
                {
                    if output.status.success() {
                        return true;
                    }
                }
            }

            false
        }
    }

    fn refresh_from_service(&mut self, service: &Option<MemoryService>) {
        if let Some(svc) = service {
            if let Ok(projects) = svc.list_projects() {
                self.projects = projects
                    .iter()
                    .map(|p| ProjectInfo {
                        name: p.project_name.clone(),
                        path: p.project_path.clone(),
                        _track_count: 0,
                    })
                    .collect();
                self.stats.project_count = self.projects.len();
            }

            let memory_limit = 200usize;
            let memories_res = if self.memory_query.trim().is_empty() {
                svc.list_memories(memory_limit)
            } else {
                svc.search_memories(self.memory_query.trim(), memory_limit)
            };
            if let Ok(memories) = memories_res {
                self.memories = memories
                    .iter()
                    .map(|m| MemoryInfo {
                        _id: m.id,
                        content: m.content.clone(),
                        category: m.category.to_string(),
                    })
                    .collect();
                if self.memories.is_empty() {
                    self.memory_state.select(Some(0));
                } else if let Some(sel) = self.memory_state.selected() {
                    if sel >= self.memories.len() {
                        self.memory_state.select(Some(self.memories.len() - 1));
                    }
                } else {
                    self.memory_state.select(Some(0));
                }
            }

            if let Ok(sessions) = svc.list_sessions() {
                self.sessions = sessions;
            }

            if let Ok(groups) = svc.list_session_groups() {
                self.groups = groups;
            }

            if let Ok(mcp_servers) = svc.list_mcp_servers() {
                // Update MCP servers list
                self.mcp_servers = mcp_servers;
            }
            if let Ok(stats) = svc.stats() {
                self.stats.memory_count = stats.memory_count;
                self.stats.track_count = stats.track_count;
            }

            // Update session statuses via tmux
            let multiplexer = TmuxMultiplexer::default();
            multiplexer.refresh_session_cache().ok();
            for session in &mut self.sessions {
                let exists = multiplexer.session_exists(&session.session_id);
                let new_status = if exists {
                    leindex_core::memory::models::SessionStatus::Running
                } else {
                    leindex_core::memory::models::SessionStatus::Terminated
                };

                if session.status != new_status {
                    session.status = new_status;
                    // Best-effort: persist status transitions so restart/resume logic is consistent.
                    let _ = svc.update_session_status(&session.session_id, new_status);
                }
            }

            // Auto-start LSPs for newly discovered sessions (best-effort)
            let _ = self.queue_lsp_autostart_for_sessions();

            // Detect LSPs for sessions based on project file types
            self.detect_session_lsps();

            self.refresh_session_entries();
            self.refresh_dash_session_entries();

            // Poll Conductor engine state
            self.conductor.poll_engine_state();


 // Poll OMP agent status for active track
 if let Some(ref manager) = self.omp_manager {
 if let Some(track_id) = &self.conductor.state.current_track {
 if let Ok(status) = manager.get_agent_status_sync(track_id) {
 // Use OmpWorkerStatus methods to check health and display status
 let worker_status: OmpWorkerStatus = status;
 if worker_status.is_healthy() {
 debug!("OMP worker healthy for track {}: model={}, uptime={}s", 
 track_id, worker_status.model, worker_status.uptime_secs);
 }
 // Update status in conductor state for display
 self.conductor.state.omp_agent_status = Some(worker_status);
 }
 }
 }
            // Fetch memories for the active track
            if let Some(track_id) = &self.conductor.state.current_track {
                if let Ok(memories) = svc.list_memories_for_track(track_id, 10) {
                    self.conductor.state.track_memories = memories;
                }
            }
        }
    }

    fn refresh_session_entries(&mut self) {
        #[derive(Clone)]
        enum SelectedKey {
            GroupPath(String),
            SessionId(String),
        }

        let selected_key = self
            .session_state
            .selected()
            .and_then(|i| self.session_entries.get(i))
            .map(|entry| match entry {
                SessionEntry::Group(g) => SelectedKey::GroupPath(g.path.clone()),
                SessionEntry::Session(s) => SelectedKey::SessionId(s.session_id.clone()),
            });

        let mut entries = Vec::new();

        // Add Groups and their sessions
        for group in &self.groups {
            entries.push(SessionEntry::Group(group.clone()));
            if group.is_expanded {
                for session in self
                    .sessions
                    .iter()
                    .filter(|s| s.group_path.as_deref() == Some(&group.path))
                {
                    entries.push(SessionEntry::Session(session.clone()));
                }
            }
        }

        // Add Uncategorized as a selectable Group if sessions exist
        let has_uncategorized = self.sessions.iter().any(|s| s.group_path.is_none());
        if has_uncategorized {
            let uncategorized_group = leindex_core::memory::models::SessionGroup {
                id: -1, // Special ID
                name: "[Uncategorized]".to_string(),
                path: "uncategorized".to_string(),
                category: None,
                is_expanded: true,
                sort_order: 9999,
                parent_id: None,
            };
            entries.push(SessionEntry::Group(uncategorized_group));
            for session in self.sessions.iter().filter(|s| s.group_path.is_none()) {
                entries.push(SessionEntry::Session(session.clone()));
            }
        }

        self.session_entries = entries;

        if self.session_entries.is_empty() {
            self.session_state.select(None);
            return;
        }

        if let Some(key) = selected_key {
            let idx = self.session_entries.iter().position(|e| match (e, &key) {
                (SessionEntry::Group(g), SelectedKey::GroupPath(path)) => &g.path == path,
                (SessionEntry::Session(s), SelectedKey::SessionId(id)) => &s.session_id == id,
                _ => false,
            });
            if let Some(idx) = idx {
                self.session_state.select(Some(idx));
                return;
            }
        }

        let idx = self
            .session_state
            .selected()
            .unwrap_or(0)
            .min(self.session_entries.len().saturating_sub(1));
        self.session_state.select(Some(idx));
    }

    fn refresh_dash_session_entries(&mut self) {
        let selected_session_id = self.dash_selected_session_id();
        let mut entries = Vec::new();

        if self.sessions.is_empty() {
            self.dash_session_entries.clear();
            self.dash_session_state.select(Some(0));
            return;
        }

        let mut sorted_sessions = self.sessions.clone();
        sorted_sessions.sort_by(|a, b| b.last_accessed_at.cmp(&a.last_accessed_at));
        let recent_sessions: Vec<_> = sorted_sessions.into_iter().take(20).collect();

        let mut seen_groups: HashSet<String> = HashSet::new();
        for sess in &recent_sessions {
            let group_key = sess
                .group_path
                .clone()
                .unwrap_or_else(|| "uncategorized".to_string());
            if seen_groups.insert(group_key.clone()) {
                entries.push(DashSessionEntry::GroupHeader {
                    group_path: group_key.clone(),
                });
                for s in recent_sessions.iter().filter(|rs| {
                    rs.group_path
                        .clone()
                        .unwrap_or_else(|| "uncategorized".to_string())
                        == group_key
                }) {
                    entries.push(DashSessionEntry::Session(s.clone()));
                }
            }
        }

        self.dash_session_entries = entries;
        if let Some(session_id) = selected_session_id {
            if let Some(idx) = self.dash_session_entries.iter().position(
                |e| matches!(e, DashSessionEntry::Session(s) if s.session_id == session_id),
            ) {
                self.dash_session_state.select(Some(idx));
                return;
            }
        }

        if let Some(selected) = self.dash_session_state.selected() {
            if selected >= self.dash_session_entries.len() {
                self.dash_select_first_session();
                return;
            }
        }

        if self
            .dash_session_state
            .selected()
            .and_then(|i| self.dash_session_entries.get(i))
            .is_some_and(|e| matches!(e, DashSessionEntry::GroupHeader { .. }))
        {
            self.dash_select_first_session();
            return;
        }

        if self.dash_session_state.selected().is_none() {
            self.dash_select_first_session();
        }
    }

    /// Refresh LSP status cache from Turso database
    ///
    /// Sets a flag to trigger async refresh in the main event loop.
    /// This avoids the Tokio panic when calling async from sync context.
    #[allow(dead_code)]
    fn refresh_lsp_status(&mut self) {
        self.refresh_lsp_status_impl(false);
    }

    /// Refresh LSP status cache from Turso database (internal implementation)
    ///
    /// Sets a flag to trigger async refresh in the main event loop.
    /// This avoids the Tokio panic when calling async from sync context.
    ///
    /// Returns true if refresh was triggered, false if throttled.
    fn refresh_lsp_status_impl(&mut self, force: bool) -> bool {
        // Only refresh every 2 seconds to avoid excessive async calls (unless forced)
        if !force && self.last_lsp_refresh.elapsed() < std::time::Duration::from_secs(2) {
            return false;
        }
        self.last_lsp_refresh = Instant::now();
        self.pending_lsp_refresh = true;
        true
    }

    /// Perform the actual LSP status refresh (async)
    ///
    /// This must be called from an async context.
    async fn do_refresh_lsp_status(&mut self) {
        let Some(storage) = self.storage_backend.clone() else {
            self.status_message = "Storage backend not available for LSP refresh".to_string();
            self.pending_lsp_refresh = false;
            return;
        };

        // Query LSP states for all sessions
        let mut new_cache: HashMap<String, Vec<(String, LspStatus)>> = HashMap::new();

        for session in &self.sessions {
            let session_id = session.session_id.clone();

            match storage.get_session_lsp_states(&session_id).await {
                Ok(states) => {
                    for state in states {
                        new_cache
                            .entry(session_id.clone())
                            .or_insert_with(Vec::new)
                            .push((state.lsp_name, state.status));
                    }
                }
                Err(_e) => {
                    // Log error but continue with other sessions
                    // Error logged but not displayed in TUI
                }
            }
        }

        // Update cache
        self.lsp_status_cache = new_cache;

        // Clamp selection to valid range
        let total_count: usize = self.lsp_status_cache.values().map(|v| v.len()).sum();
        if let Some(selected) = self.lsp_state.selected() {
            if selected >= total_count && total_count > 0 {
                self.lsp_state.select(Some(total_count.saturating_sub(1)));
            } else if total_count == 0 {
                self.lsp_state.select(None);
            }
        }

        self.pending_lsp_refresh = false;

        // Update the aggregated status summary
        self.compute_lsp_status_summary();
    }

    /// Compute aggregated LSP status summary from cache
    fn compute_lsp_status_summary(&mut self) {
        let mut summary = crate::state::LspStatusSummary::default();

        for lsp_states in self.lsp_status_cache.values() {
            for (_lsp_name, status) in lsp_states {
                summary.total_lsps += 1;
                match status {
                    LspStatus::Running => summary.running += 1,
                    LspStatus::Stopped => summary.stopped += 1,
                    LspStatus::Error => summary.errors += 1,
                    LspStatus::Starting => summary.starting += 1,
                }
            }
        }

        // Count total errors/warnings from diagnostic summaries
        for diag in &self.lsp_diagnostic_summaries {
            summary.total_errors += diag.counts.errors;
            summary.total_warnings += diag.counts.warnings;
        }

        self.lsp_status_summary = summary;
    }

    /// Detect LSPs for all sessions based on project file types (doesn't start them)
    fn detect_session_lsps(&mut self) {
        self.lsp_detected_cache.clear();

        for session in &self.sessions {
            if session.project_path.trim().is_empty() {
                continue;
            }

            let path = std::path::Path::new(&session.project_path);
            if !path.exists() {
                continue;
            }

            let mut detected: Vec<String> = Vec::new();

            // Quick scan for file extensions (depth-limited)
            let max_depth = 2;
            let mut dirs_to_visit: Vec<(std::path::PathBuf, usize)> =
                vec![(path.to_path_buf(), 0)];

            while let Some((current_dir, depth)) = dirs_to_visit.pop() {
                if depth > max_depth {
                    continue;
                }

                let Ok(entries) = std::fs::read_dir(&current_dir) else {
                    continue;
                };

                for entry in entries.flatten() {
                    let Ok(ft) = entry.file_type() else {
                        continue;
                    };

                    if ft.is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            if !name.starts_with('.')
                                && name != "node_modules"
                                && name != "target"
                                && name != "build"
                                && name != "dist"
                            {
                                dirs_to_visit.push((entry.path(), depth + 1));
                            }
                        }
                    } else if ft.is_file() {
                        if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                            match ext {
                                "rs" => {
                                    if !detected.contains(&"rust-analyzer".to_string()) {
                                        detected.push("rust-analyzer".to_string());
                                    }
                                }
                                "py" => {
                                    if !detected.contains(&"ruff".to_string()) {
                                        detected.push("ruff".to_string());
                                    }
                                }
                                "ts" | "tsx" | "js" | "jsx" => {
                                    if !detected.contains(&"typescript-language-server".to_string()) {
                                        detected.push("typescript-language-server".to_string());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            // Sort for consistent ordering
            detected.sort();
            self.lsp_detected_cache
                .insert(session.session_id.clone(), detected);
        }
    }

    /// Auto-start LSPs for sessions that haven't been scanned yet.
    fn queue_lsp_autostart_for_sessions(&mut self) -> bool {
        let Some(lsp_manager) = self.lsp_manager.clone() else {
            return false;
        };

        let known_ids: HashSet<String> =
            self.sessions.iter().map(|s| s.session_id.clone()).collect();
        self.lsp_autostarted_sessions
            .retain(|id| known_ids.contains(id));

        let mut scheduled = false;
        for session in &self.sessions {
            if self.lsp_autostarted_sessions.contains(&session.session_id) {
                continue;
            }

            if session.project_path.trim().is_empty() {
                continue;
            }

            if matches!(
                session.status,
                leindex_core::memory::models::SessionStatus::Terminated
                    | leindex_core::memory::models::SessionStatus::Completed
            ) {
                continue;
            }

            let session_id = session.session_id.clone();
            let project_path = std::path::PathBuf::from(session.project_path.clone());

            self.lsp_autostarted_sessions.insert(session_id.clone());
            scheduled = true;

            let manager = lsp_manager.clone();
            tokio::spawn(async move {
                let _ = manager
                    .auto_start_lsps_for_session(&session_id, &project_path)
                    .await;
            });
        }

        if scheduled {
            self.refresh_lsp_status_impl(true);
        }

        scheduled
    }

    /// Get selected LSP from cache
    ///
    /// Builds the LSP list in the same order as render_lsps (session order)
    /// to ensure the correct LSP is selected for toggle/restart/logs operations.
    fn get_selected_lsp(&self) -> Option<(String, String, LspStatus)> {
        let selected_index = self.lsp_state.selected()?;

        // Build LSP entries list in session order (same as render_lsps)
        let mut all_lsps: Vec<(String, String, LspStatus)> = Vec::new();
        for session in &self.sessions {
            if let Some(lsp_states) = self.lsp_status_cache.get(&session.session_id) {
                for (lsp_name, status) in lsp_states {
                    all_lsps.push((session.session_id.clone(), lsp_name.clone(), *status));
                }
            }
        }

        all_lsps.get(selected_index).cloned()
    }

    /// Map LSP name to LspType
    fn lsp_name_to_type(lsp_name: &str) -> Option<LspType> {
        match lsp_name {
            "rust-analyzer" => Some(LspType::Rust),
            "ruff" => Some(LspType::Python),
            "typescript-language-server" => Some(LspType::TypeScript),
            _ => None,
        }
    }

    /// Toggle LSP start/stop
    ///
    /// This sets a pending flag and the actual operation is performed in the async event loop.
    fn toggle_lsp(&mut self, session_id: &str, lsp_name: &str, status: LspStatus) {
        let Some(lsp_type) = Self::lsp_name_to_type(lsp_name) else {
            self.status_message = format!("Unknown LSP: {}", lsp_name);
            return;
        };

        let Some(lsp_manager) = self.lsp_manager.clone() else {
            self.status_message = "LSP Manager not available".to_string();
            return;
        };

        let project_path = self
            .sessions
            .iter()
            .find(|s| s.session_id == session_id)
            .map(|s| s.project_path.clone())
            .unwrap_or_else(|| ".".to_string());

        let session_id = session_id.to_string();
        let lsp_name = lsp_name.to_string();

        // Spawn the operation in the background
        tokio::spawn(async move {
            let result = match status {
                LspStatus::Stopped | LspStatus::Error => {
                    // Start the LSP with MCP bridge for diagnostics
                    let (start_result, _bridge_pid) = lsp_manager
                        .start_lsp_with_mcp_bridge(
                            &session_id,
                            lsp_type,
                            std::path::Path::new(&project_path),
                            None,
                        )
                        .await;
                    start_result
                }
                LspStatus::Running | LspStatus::Starting => {
                    // Stop the LSP and MCP bridge
                    lsp_manager
                        .stop_lsp_with_mcp_bridge(&session_id, lsp_type, 0)
                        .await
                }
            };

            result
        });

        // Update status message optimistically - actual status will be reflected on next refresh
        let action_msg = match status {
            LspStatus::Stopped | LspStatus::Error => "Starting",
            LspStatus::Running | LspStatus::Starting => "Stopping",
        };
        self.status_message = format!("{} '{}'... (press 'r' to refresh)", action_msg, lsp_name);

        // Trigger a delayed refresh
        self.refresh_lsp_status_impl(true);
    }

    /// Restart LSP
    ///
    /// This spawns the operation in the background and triggers a refresh.
    fn restart_lsp(&mut self, session_id: &str, lsp_name: &str) {
        let Some(lsp_type) = Self::lsp_name_to_type(lsp_name) else {
            self.status_message = format!("Unknown LSP: {}", lsp_name);
            return;
        };

        let Some(lsp_manager) = self.lsp_manager.clone() else {
            self.status_message = "LSP Manager not available".to_string();
            return;
        };

        let project_path = self
            .sessions
            .iter()
            .find(|s| s.session_id == session_id)
            .map(|s| s.project_path.clone())
            .unwrap_or_else(|| ".".to_string());

        let session_id = session_id.to_string();
        let lsp_name = lsp_name.to_string();

        // Spawn the operation in the background
        tokio::spawn(async move {
            let _ = lsp_manager
                .stop_lsp_with_mcp_bridge(&session_id, lsp_type, 0)
                .await;
            let _ = lsp_manager
                .start_lsp_with_mcp_bridge(
                    &session_id,
                    lsp_type,
                    std::path::Path::new(&project_path),
                    None,
                )
                .await;
        });

        // Update status message optimistically - actual status will be reflected on next refresh
        self.status_message = format!("Restarting '{}'... (press 'r' to refresh)", lsp_name);

        // Trigger a delayed refresh
        self.refresh_lsp_status_impl(true);
    }

    /// Read LSP logs for the specified session and LSP
    fn read_lsp_logs(&mut self, session_id: &str, lsp_name: &str) {
        // Sanitize lsp_name for use in filename (replace spaces with dashes)
        let safe_lsp_name = lsp_name.replace(' ', "-").to_lowercase();

        // Sanitize session_id to prevent path traversal attacks
        // Only allow alphanumeric characters, hyphens, and underscores
        let safe_session_id: String = session_id
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();

        // If sanitization removed characters, use a hash for safety
        let safe_session_id =
            if safe_session_id.is_empty() || safe_session_id.len() != session_id.len() {
                // Use a hash for safety - std::hash::DefaultHasher
                let mut hasher = DefaultHasher::new();
                session_id.hash(&mut hasher);
                format!("session_{:x}", hasher.finish())
            } else {
                safe_session_id
            };

        // Check common log locations
        let log_paths = vec![
            format!("/tmp/{}-{}.log", safe_lsp_name, safe_session_id),
            format!("/tmp/maestro-lsp-{}-{}.log", safe_session_id, safe_lsp_name),
            format!("/tmp/maestro-lsp-{}.log", safe_session_id),
            format!("/tmp/maestro-lsp-{}-stdout.log", safe_session_id),
            format!("/tmp/maestro-lsp-{}-stderr.log", safe_session_id),
            format!("/tmp/{}.log", safe_lsp_name),
        ];

        for path in log_paths {
            if let Ok(content) = std::fs::read_to_string(&path) {
                self.lsp_log_content = content;
                self.lsp_log_source = Some((session_id.to_string(), lsp_name.to_string()));
                self.lsp_log_scroll = 0;
                return;
            }
        }

        // No log file found - provide helpful message
        self.lsp_log_content = format!(
            "No logs available for LSP '{}' in session '{}'.\n\n\
             LSP logs are not currently being captured to files.\n\n\
             To enable LSP logging, you can:\n\
             - Start the LSP with output redirection to a log file\n\
             - Check the LSP configuration for logging options\n\
             - Use tools like journalctl or dmesg for system-level logs\n\n\
             Common log locations checked:\n\
             - /tmp/{}-{}.log\n\
             - /tmp/maestro-lsp-{}-{}.log\n\
             - /tmp/maestro-lsp-{}.log\n\
             - /tmp/maestro-lsp-{{}}-stdout.log\n\
             - /tmp/maestro-lsp-{{}}-stderr.log",
            lsp_name,
            session_id,
            safe_lsp_name,
            safe_session_id,
            safe_session_id,
            safe_lsp_name,
            safe_session_id
        );
        self.lsp_log_source = Some((session_id.to_string(), lsp_name.to_string()));
        self.lsp_log_scroll = 0;
    }
}

fn suspend_fullscreen_app<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    // If we're already inside a Zellij pane, switching the terminal's alternate
    // screen can cause rendering glitches. In that case, let the spawned app
    // manage the terminal as-is.
    if std::env::var("ZELLIJ").is_ok() {
        return Ok(());
    }
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn resume_fullscreen_app<B: Backend>(_terminal: &mut Terminal<B>) -> Result<()> {
    if std::env::var("ZELLIJ").is_ok() {
        return Ok(());
    }
    execute!(io::stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    service: Option<MemoryService>,
    mcp_pool: Option<Arc<McpPool>>,
    storage_backend: Option<Arc<TursoStorageBackend>>,
) -> Result<()> {
    let mut app = App::new(service.as_ref(), mcp_pool, storage_backend);
    let mut last_refresh = std::time::Instant::now();

    loop {
        terminal.draw(|frame| ui(frame, &mut app))?;

        // Handle MCP refresh task shutdown on quit
        if app.should_quit {
            if let Some(ref task) = app.mcp_refresh_task {
                task.abort();
            }
            break;
        }

        // Handle pending LSP refresh (async operation)
        if app.pending_lsp_refresh {
            app.do_refresh_lsp_status().await;
        }

        // Periodic refresh (every 500ms)
        if last_refresh.elapsed() >= std::time::Duration::from_millis(500) {
            app.refresh_from_service(&service);
            // Note: LSP status is no longer auto-refreshed periodically
            // It's only refreshed on explicit user action (press 'r')
            last_refresh = std::time::Instant::now();
        }

        // Fetch preview for selected session
        if app.tab_index == 1
            && app.last_preview_refresh.elapsed() >= std::time::Duration::from_millis(200)
        {
            app.last_preview_refresh = Instant::now();

            if let Some(i) = app.session_state.selected() {
                if let Some(SessionEntry::Session(s)) = app.session_entries.get(i).cloned() {
                    match TmuxMultiplexer::get_pane_content(&s.session_id, 25) {
                        Ok(content) => app.session_preview_content = content,
                        Err(_) => {
                            app.session_preview_content =
                                session_log_tail(&s.session_id, 200).unwrap_or_default();
                        }
                    }
                } else {
                    app.session_preview_content.clear();
                }
            } else {
                app.session_preview_content.clear();
            }
        }

        app.frame_count = app.frame_count.wrapping_add(1);

        // High FPS polling (5ms) for 180Hz monitors, floor of 60fps
        if event::poll(std::time::Duration::from_millis(5))? {
            if let Event::Key(key) = event::read()? {
                // Some terminals report Enter as Repeat; treat Press+Repeat as actionable input.
                if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                    if app.input_mode != InputMode::Normal {
                        match key.code {
                            KeyCode::Enter | KeyCode::Char('\n') | KeyCode::Char('\r') => {
                                match app.input_mode {
                                    InputMode::NewSessionTitle => {
                                        app.input_mode = InputMode::NewSessionPath
                                    }
                                    InputMode::NewSessionPath => {
                                        app.input_mode = InputMode::NewSessionTool
                                    }
                                    InputMode::NewSessionTool => {
                                        app.is_spawning = true;
                                        app.status_message =
                                            format!("Spawning {} session...", app.new_session_tool);
                                        // let _ = terminal.draw(|frame| ui(frame, \u0026mut app));

                                        if let Some(svc) = service.as_ref() {
                                            let manager = match leindex_core::memory::session_manager::SessionManager::new(svc.clone()) {
                                                Ok(m) => m,
                                                Err(e) => {
                                                    app.status_message = format!("Failed to create session manager: {}", e);
                                                    app.is_spawning = false;
                                                    continue;
                                                }
                                            };
                                            match manager.create_session(
                                                &app.new_session_title,
                                                &app.new_session_path,
                                                &app.new_session_tool,
                                                None,
                                                None,
                                            ) {
                                                Ok(session) => {
                                                    // Ensure project is registered in memory pipeline
                                                    let project_name =
                                                        std::path::Path::new(&session.project_path)
                                                            .file_name()
                                                            .and_then(|name| name.to_str())
                                                            .unwrap_or(&session.title)
                                                            .to_string();
                                                    let _ = svc.get_or_create_project(
                                                        &session.project_path,
                                                        &project_name,
                                                    );
                                                    let _ = svc
                                                        .update_last_accessed(&session.session_id);
                                                    let _ = svc.sync_mcp_servers_from_system();
                                                    let _ = svc.sync_memories_from_system();

                                                    app.sessions.push(session.clone());
                                                    let _ = app.queue_lsp_autostart_for_sessions();
                                                    app.refresh_session_entries();
                                                    app.refresh_dash_session_entries();
                                                    app.status_message = format!("Session '{}' created. Press Enter on Sessions tab to attach.", session.title);
                                                    app.tab_index = 1;
                                                    let new_idx = app
                                                        .session_entries
                                                        .iter()
                                                        .position(|e| {
                                                            if let SessionEntry::Session(s) = e {
                                                                s.session_id == session.session_id
                                                            } else {
                                                                false
                                                            }
                                                        })
                                                        .unwrap_or(0);
                                                    app.session_state.select(Some(new_idx));
                                                    app.refresh_from_service(&service);
                                                }
                                                Err(e) => {
                                                    app.status_message = format!("Error: {}", e);
                                                    let _ =
                                                        terminal.draw(|frame| ui(frame, &mut app));
                                                    std::thread::sleep(
                                                        std::time::Duration::from_secs(2),
                                                    );
                                                }
                                            }
                                        } else {
                                            app.status_message =
                                                "Error: Memory service not available".to_string();
                                            let _ = terminal.draw(|frame| ui(frame, &mut app));
                                            std::thread::sleep(std::time::Duration::from_secs(2));
                                        }
                                        app.is_spawning = false;
                                        app.input_mode = InputMode::Normal;
                                        app.new_session_title.clear();
                                        app.new_session_path.clear();
                                    }

                                    InputMode::SessionSwitcher => {
                                        if let Some(i) = app.switcher_state.selected() {
                                            if let Some(session) = app.sessions.get(i).cloned() {
                                                app.status_message = format!(
                                                    "Attaching to '{}'... (Ctrl+B d to detach)",
                                                    session.title
                                                );
                                                let _ = terminal.draw(|frame| ui(frame, &mut app));
                                                let _ = suspend_fullscreen_app(terminal);
                                                let res =
                                                    TmuxMultiplexer::attach(&session.session_id);
                                                let _ = resume_fullscreen_app(terminal);
                                                let _ = terminal.clear();
                                                app.status_message = match res {
                                                    Ok(()) => {
                                                        format!("Returned from '{}'", session.title)
                                                    }
                                                    Err(e) => format!("Attach failed: {}", e),
                                                };
                                            }
                                        }
                                        app.input_mode = InputMode::Normal;
                                    }

                                    InputMode::RenameGroup | InputMode::RenameGroupCategory => {
                                        let Some(svc) = service.as_ref() else {
                                            app.status_message =
                                                "Error: Memory service not available".to_string();
                                            app.input_mode = InputMode::Normal;
                                            continue;
                                        };

                                        let Some(old_path) = app.target_group_path.clone() else {
                                            app.status_message =
                                                "Error: No group selected to rename".to_string();
                                            app.input_mode = InputMode::Normal;
                                            continue;
                                        };

                                        let clean_name = app.rename_buffer.trim();
                                        if clean_name.is_empty() {
                                            app.status_message =
                                                "Group name cannot be empty".to_string();
                                            app.input_mode = InputMode::RenameGroup;
                                            continue;
                                        }

                                        let category = if app.new_group_category.trim().is_empty() {
                                            None
                                        } else {
                                            Some(app.new_group_category.trim().to_string())
                                        };

                                        let mut select_group_path: Option<String> = None;
                                        let mut rename_ok = false;

                                        if old_path == "uncategorized" {
                                            let new_path = format!(
                                                "/{}",
                                                clean_name.to_lowercase().replace(' ', "_")
                                            );

                                            let group =
                                                leindex_core::memory::models::SessionGroup {
                                                    id: 0,
                                                    name: clean_name.to_string(),
                                                    path: new_path.clone(),
                                                    category: category.clone(),
                                                    is_expanded: true,
                                                    sort_order: 0,
                                                    parent_id: None,
                                                };

                                            if let Err(e) = svc.get_or_create_session_group(group) {
                                                app.status_message =
                                                    format!("Error creating group: {}", e);
                                                app.input_mode = InputMode::RenameGroup;
                                            } else {
                                                let _ =
                                                    svc.update_group_category(&new_path, category);
                                                let _ = svc.update_group_expansion(&new_path, true);

                                                let mut moved = 0usize;
                                                if let Ok(sessions) = svc.list_sessions() {
                                                    for s in sessions {
                                                        if s.group_path.is_none() {
                                                            if svc
                                                                .update_session_group(
                                                                    &s.session_id,
                                                                    Some(new_path.clone()),
                                                                )
                                                                .is_ok()
                                                            {
                                                                moved += 1;
                                                            }
                                                        }
                                                    }
                                                }

                                                app.status_message = format!(
                                                    "Created group '{}' and moved {} sessions.",
                                                    clean_name, moved
                                                );
                                                select_group_path = Some(new_path);
                                                rename_ok = true;
                                            }
                                        } else {
                                            match svc.rename_group(&old_path, clean_name) {
                                                Ok(new_path) => {
                                                    // Extra safety: ensure sessions remain associated even if older DB
                                                    // rows had mismatched group_path values.
                                                    if let Ok(sessions) = svc.list_sessions() {
                                                        for s in sessions {
                                                            if s.group_path.as_deref()
                                                                == Some(old_path.as_str())
                                                            {
                                                                let _ = svc.update_session_group(
                                                                    &s.session_id,
                                                                    Some(new_path.clone()),
                                                                );
                                                            }
                                                        }
                                                    }

                                                    let _ = svc
                                                        .update_group_category(&new_path, category);
                                                    let _ =
                                                        svc.update_group_expansion(&new_path, true);
                                                    app.status_message =
                                                        format!("Group '{}' updated", clean_name);
                                                    select_group_path = Some(new_path);
                                                    rename_ok = true;
                                                }
                                                Err(e) => {
                                                    app.status_message =
                                                        format!("Error renaming group: {}", e);
                                                    app.input_mode = InputMode::RenameGroup;
                                                }
                                            }
                                        }

                                        if rename_ok {
                                            // Reload from the database so path changes are reflected in-app.
                                            if let Ok(groups) = svc.list_session_groups() {
                                                app.groups = groups;
                                            }
                                            if let Ok(sessions) = svc.list_sessions() {
                                                app.sessions = sessions;
                                            }

                                            if let Some(ref group_path) = select_group_path {
                                                if let Some(group) = app
                                                    .groups
                                                    .iter_mut()
                                                    .find(|g| g.path == *group_path)
                                                {
                                                    group.is_expanded = true;
                                                }
                                                let _ =
                                                    svc.update_group_expansion(group_path, true);
                                            }

                                            app.refresh_session_entries();
                                            app.refresh_dash_session_entries();

                                            if let Some(group_path) = select_group_path {
                                                if let Some(idx) = app.session_entries.iter().position(|e| {
                                                    matches!(e, SessionEntry::Group(g) if g.path == group_path)
                                                }) {
                                                    app.session_state.select(Some(idx));
                                                }
                                            }

                                            app.target_group_path = None;
                                            app.rename_buffer.clear();
                                            app.new_group_category.clear();
                                            app.input_mode = InputMode::Normal;
                                        }
                                    }
                                    InputMode::ForkSession => {
                                        if let Some(svc) = service.as_ref() {
                                            if let Some(id) = app.target_session_id.take() {
                                                if let Some(orig) =
                                                    app.sessions.iter().find(|s| s.session_id == id)
                                                {
                                                    let manager = match leindex_core::memory::session_manager::SessionManager::new(svc.clone()) {
                                                        Ok(m) => m,
                                                        Err(e) => {
                                                            app.status_message = format!("Failed to create session manager: {}", e);
                                                            app.input_mode = InputMode::Normal;
                                                            continue;
                                                        }
                                                    };
                                                    let _ = manager.fork_session(
                                                        &id,
                                                        &app.rename_buffer,
                                                        orig,
                                                    );
                                                    app.status_message = format!(
                                                        "Session forked as {}",
                                                        app.rename_buffer
                                                    );
                                                    if let Ok(sessions) = svc.list_sessions() {
                                                        app.sessions = sessions;
                                                    }
                                                }
                                            }
                                        }
                                        app.refresh_session_entries();
                                        app.refresh_dash_session_entries();
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::KillConfirm | InputMode::DeleteConfirm => {
                                        if let Some(svc) = service.as_ref() {
                                            if let Some(id) = app.target_session_id.take() {
                                                let manager = match leindex_core::memory::session_manager::SessionManager::new(svc.clone()) {
                                                    Ok(m) => m,
                                                    Err(e) => {
                                                        app.status_message = format!("Failed to create session manager: {}", e);
                                                        continue;
                                                    }
                                                };
                                                match manager.kill_session(&id) {
                                                    Ok(()) => {
                                                        if app.input_mode
                                                            == InputMode::DeleteConfirm
                                                        {
                                                            let _ = svc.delete_session(&id);
                                                            app.status_message =
                                                                "Session deleted".to_string();
                                                        } else {
                                                            app.status_message =
                                                                "Session killed".to_string();
                                                        }
                                                        if let Ok(sessions) = svc.list_sessions() {
                                                            app.sessions = sessions;
                                                        }
                                                    }
                                                    Err(e) => {
                                                        app.status_message =
                                                            format!("Kill failed: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                        app.refresh_session_entries();
                                        app.refresh_dash_session_entries();
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::AnalysisPrompt => {
                                        let input = app.analysis_input.trim().to_string();
                                        if !input.is_empty() {
                                            app.analysis_history.push(format!("> {}", input));
                                            let tokens: Vec<&str> =
                                                input.split_whitespace().collect();
                                            let cmd = tokens
                                                .first()
                                                .map(|s| s.to_lowercase())
                                                .unwrap_or_default();

                                            let mut push_block = |text: &str| {
                                                for line in text.lines() {
                                                    app.analysis_history.push(line.to_string());
                                                }
                                                const MAX_LINES: usize = 500;
                                                if app.analysis_history.len() > MAX_LINES {
                                                    let drain =
                                                        app.analysis_history.len() - MAX_LINES;
                                                    app.analysis_history.drain(0..drain);
                                                }
                                            };

                                            let parse_phase_opts = || {
                                                let mut opts =
                                                    leindex_core::five_phase::PhaseOptions::new(
                                                        std::path::PathBuf::from("."),
                                                    );

                                                let mut path_set = false;
                                                let mut i = 1usize;
                                                while i < tokens.len() {
                                                    let t = tokens[i];
                                                    match t {
                                                        "--mode" | "-m" => {
                                                            if let Some(v) = tokens.get(i + 1) {
                                                                opts.mode = leindex_core::token_format::FormatMode::from_str(v);
                                                                i += 1;
                                                            }
                                                        }
                                                        "--files" | "--max-files" | "-n" => {
                                                            if let Some(v) = tokens.get(i + 1) {
                                                                if let Ok(n) = v.parse::<usize>() {
                                                                    opts.max_files = n.max(1);
                                                                }
                                                                i += 1;
                                                            }
                                                        }
                                                        "--focus-files" => {
                                                            if let Some(v) = tokens.get(i + 1) {
                                                                if let Ok(n) = v.parse::<usize>() {
                                                                    opts.max_focus_files = n.max(1);
                                                                }
                                                                i += 1;
                                                            }
                                                        }
                                                        "--top" => {
                                                            if let Some(v) = tokens.get(i + 1) {
                                                                if let Ok(n) = v.parse::<usize>() {
                                                                    opts.top_n = n.max(1);
                                                                }
                                                                i += 1;
                                                            }
                                                        }
                                                        "--chars" | "--max-chars" => {
                                                            if let Some(v) = tokens.get(i + 1) {
                                                                if let Ok(n) = v.parse::<usize>() {
                                                                    opts.max_output_chars = n.max(200);
                                                                }
                                                                i += 1;
                                                            }
                                                        }
                                                        t if t.starts_with("--mode=") => {
                                                            if let Some((_, v)) = t.split_once('=') {
                                                                opts.mode =
                                                                    leindex_core::token_format::FormatMode::from_str(v);
                                                            }
                                                        }
                                                        t if t.starts_with("--files=") => {
                                                            if let Some((_, v)) = t.split_once('=') {
                                                                if let Ok(n) = v.parse::<usize>() {
                                                                    opts.max_files = n.max(1);
                                                                }
                                                            }
                                                        }
                                                        t if t.starts_with("--focus-files=") => {
                                                            if let Some((_, v)) = t.split_once('=') {
                                                                if let Ok(n) = v.parse::<usize>() {
                                                                    opts.max_focus_files = n.max(1);
                                                                }
                                                            }
                                                        }
                                                        t if t.starts_with("--top=") => {
                                                            if let Some((_, v)) = t.split_once('=') {
                                                                if let Ok(n) = v.parse::<usize>() {
                                                                    opts.top_n = n.max(1);
                                                                }
                                                            }
                                                        }
                                                        t if t.starts_with("--chars=") => {
                                                            if let Some((_, v)) = t.split_once('=') {
                                                                if let Ok(n) = v.parse::<usize>() {
                                                                    opts.max_output_chars = n.max(200);
                                                                }
                                                            }
                                                        }
                                                        "ultra" | "u" => {
                                                            opts.mode = leindex_core::token_format::FormatMode::Ultra
                                                        }
                                                        "balanced" | "b" => {
                                                            opts.mode =
                                                                leindex_core::token_format::FormatMode::Balanced
                                                        }
                                                        "verbose" | "v" => {
                                                            opts.mode =
                                                                leindex_core::token_format::FormatMode::Verbose
                                                        }
                                                        t if !t.starts_with('-') && !path_set => {
                                                            opts.root = std::path::PathBuf::from(t);
                                                            path_set = true;
                                                        }
                                                        _ => {}
                                                    }
                                                    i += 1;
                                                }

                                                opts
                                            };

                                            match cmd.as_str() {
                                                "/phase1" | "/p1" => {
                                                    let opts = parse_phase_opts();
                                                    match leindex_core::five_phase::phase1_structural_scan(&opts)
                                                    {
                                                        Ok(out) => push_block(&out),
                                                        Err(e) => push_block(&format!(
                                                            "Error running /phase1: {}",
                                                            e
                                                        )),
                                                    }
                                                }
                                                "/phase2" | "/p2" => {
                                                    let opts = parse_phase_opts();
                                                    match leindex_core::five_phase::phase2_dependency_map(&opts) {
                                                        Ok(out) => push_block(&out),
                                                        Err(e) => push_block(&format!(
                                                            "Error running /phase2: {}",
                                                            e
                                                        )),
                                                    }
                                                }
                                                "/phase3" | "/p3" => {
                                                    let opts = parse_phase_opts();
                                                    match leindex_core::five_phase::phase3_logic_flow(&opts) {
                                                        Ok(out) => push_block(&out),
                                                        Err(e) => push_block(&format!(
                                                            "Error running /phase3: {}",
                                                            e
                                                        )),
                                                    }
                                                }
                                                "/phase4" | "/p4" => {
                                                    let opts = parse_phase_opts();
                                                    match leindex_core::five_phase::phase4_critical_path(&opts) {
                                                        Ok(out) => push_block(&out),
                                                        Err(e) => push_block(&format!(
                                                            "Error running /phase4: {}",
                                                            e
                                                        )),
                                                    }
                                                }
                                                "/phase5" | "/p5" => {
                                                    let opts = parse_phase_opts();
                                                    match leindex_core::five_phase::phase5_optimization_report(&opts) {
                                                        Ok(out) => push_block(&out),
                                                        Err(e) => push_block(&format!(
                                                            "Error running /phase5: {}",
                                                            e
                                                        )),
                                                    }
                                                }
                                                "analyze" => push_block(
                                                    "Tip: use /phase1 ./path (ultra, token-efficient) for structural scan.",
                                                ),
                                                "scan" => push_block(
                                                    "Tip: memory scan is available via CLI: `maestro memory scan <paths...>`",
                                                ),
                                                "stats" => push_block("Tip: /phase4 and /phase5 include complexity hotspots."),
                                                "help" | "/help" => {
                                                    push_block("AVAILABLE COMMANDS:");
                                                    push_block("  /phase1 [path] [--mode ultra|balanced|verbose] [--files N] [--chars N]");
                                                    push_block("  /phase2 [path] [--mode ...] [--files N]");
                                                    push_block("  /phase3 [path] [--mode ...] [--focus-files N]");
                                                    push_block("  /phase4 [path] [--top N] [--files N]");
                                                    push_block("  /phase5 [path] [--files N]");
                                                }
                                                _ => push_block(&format!(
                                                    "Unknown command: {}. Try 'help' or '/help'.",
                                                    input
                                                )),
                                            }
                                            app.analysis_input.clear();
                                        } else {
                                            app.input_mode = InputMode::Normal;
                                        }
                                    }
                                    InputMode::MemorySearch => {
                                        app.input_mode = InputMode::Normal;
                                        app.refresh_from_service(&service);
                                    }

                                    InputMode::NewMemoryContent => {
                                        // Move to category input
                                        if app.new_memory_content.trim().is_empty() {
                                            app.status_message = "Memory content cannot be empty".to_string();
                                        } else {
                                            app.input_mode = InputMode::NewMemoryCategory;
                                            app.status_message = "Enter category (or press Enter for 'general')".to_string();
                                        }
                                    }

                                    InputMode::NewMemoryCategory => {
                                        // Create the memory
                                        let Some(svc) = service.as_ref() else {
                                            app.status_message = "Error: Memory service not available".to_string();
                                            app.input_mode = InputMode::Normal;
                                            continue;
                                        };

                                        let category = if app.new_memory_category.trim().is_empty() {
                                            "general".to_string()
                                        } else {
                                            app.new_memory_category.trim().to_string()
                                        };

                                        // Parse category to MemoryCategory enum
                                        let mem_category = match category.to_lowercase().as_str() {
                                            "knowledge" => leindex_core::memory::models::MemoryCategory::Knowledge,
                                            "preference" | "preferences" => leindex_core::memory::models::MemoryCategory::Preference,
                                            "specification" | "spec" | "specs" => leindex_core::memory::models::MemoryCategory::Specification,
                                            "fact" => leindex_core::memory::models::MemoryCategory::Fact,
                                            "pattern" => leindex_core::memory::models::MemoryCategory::Pattern,
                                            "decision" => leindex_core::memory::models::MemoryCategory::Decision,
                                            "context" => leindex_core::memory::models::MemoryCategory::Context,
                                            "temporary" | "temp" => leindex_core::memory::models::MemoryCategory::Temporary,
                                            "observation" => leindex_core::memory::models::MemoryCategory::Observation,
                                            _ => leindex_core::memory::models::MemoryCategory::General,
                                        };

                                        match svc.store_memory(app.new_memory_content.trim(), mem_category) {
                                            Ok(_) => {
                                                app.status_message = format!("Memory created with category '{}'", category);
                                                app.refresh_from_service(&service);
                                            }
                                            Err(e) => {
                                                app.status_message = format!("Failed to create memory: {}", e);
                                            }
                                        }

                                        app.new_memory_content.clear();
                                        app.new_memory_category.clear();
                                        app.input_mode = InputMode::Normal;
                                    }

                                    InputMode::McpMenu => {
                                        let Some(svc) = service.as_ref() else {
                                            app.status_message =
                                                "Error: Memory service not available".to_string();
                                            app.input_mode = InputMode::Normal;
                                            continue;
                                        };
                                        let Some(name) = app.target_mcp_name.clone() else {
                                            app.input_mode = InputMode::Normal;
                                            continue;
                                        };

                                        let server = app
                                            .mcp_servers
                                            .iter()
                                            .find(|s| s.name == name)
                                            .cloned();

                                        match app.mcp_menu_option {
                                            McpOption::Start => {
                                                let Some(pool) = app.mcp_pool.clone() else {
                                                    app.status_message =
                                                        "MCP pool not available".to_string();
                                                    app.input_mode = InputMode::Normal;
                                                    app.target_mcp_name = None;
                                                    continue;
                                                };
                                                let Some(server) = server else {
                                                    app.status_message =
                                                        "MCP server not found".to_string();
                                                    app.input_mode = InputMode::Normal;
                                                    app.target_mcp_name = None;
                                                    continue;
                                                };

                                                match pool.start_server_record(&server).await {
                                                    Ok(socket) => {
                                                        app.status_message = format!(
                                                            "Started MCP '{}' at {}",
                                                            name, socket
                                                        );
                                                        // Refresh MCP list to show updated status
                                                        if let Ok(mcp_list) = svc.list_mcp_servers()
                                                        {
                                                            app.mcp_servers = mcp_list;
                                                        }
                                                    }
                                                    Err(e) => {
                                                        app.status_message =
                                                            format!("Start failed: {}", e);
                                                        // Refresh MCP list to show updated status
                                                        if let Ok(mcp_list) = svc.list_mcp_servers()
                                                        {
                                                            app.mcp_servers = mcp_list;
                                                        }
                                                    }
                                                }
                                            }
                                            McpOption::Stop => {
                                                let Some(pool) = app.mcp_pool.clone() else {
                                                    app.status_message =
                                                        "MCP pool not available".to_string();
                                                    app.input_mode = InputMode::Normal;
                                                    app.target_mcp_name = None;
                                                    continue;
                                                };
                                                if let Err(e) = pool.stop_server(&name).await {
                                                    app.status_message =
                                                        format!("Stop failed: {}", e);
                                                } else {
                                                    app.status_message =
                                                        format!("Stopped MCP '{}'", name);
                                                    // Refresh MCP list to show updated status
                                                    if let Ok(mcp_list) = svc.list_mcp_servers() {
                                                        app.mcp_servers = mcp_list;
                                                    }
                                                }
                                            }
                                            McpOption::Pause => {
                                                app.status_message =
                                                    format!("MCP '{}' pause not implemented", name);
                                            }
                                            McpOption::Logs => {
                                                let log_path = McpPool::log_path_for(&name);
                                                let content = std::fs::read_to_string(&log_path)
                                                    .unwrap_or_default();
                                                app.mcp_log_lines = content
                                                    .lines()
                                                    .rev()
                                                    .take(500)
                                                    .collect::<Vec<_>>()
                                                    .into_iter()
                                                    .rev()
                                                    .map(|s| s.to_string())
                                                    .collect();
                                                app.mcp_log_scroll = 0;
                                                app.input_mode = InputMode::McpLogs;
                                                continue;
                                            }
                                            McpOption::Add => {
                                                match svc.sync_mcp_servers_from_system() {
                                                    Ok(n) => {
                                                        // Also start all discovered servers
                                                        if let Some(pool) = app.mcp_pool.clone() {
                                                            let _ = pool.start_all_from_db().await;
                                                        }
                                                        app.status_message = format!(
                                                            "Discovered & synced {} MCP server(s) from system configs",
                                                            n
                                                        );
                                                    }
                                                    Err(e) => {
                                                        app.status_message =
                                                            format!("Discovery failed: {}", e)
                                                    }
                                                }
                                            }
                                            McpOption::Remove => {
                                                if let Some(pool) = app.mcp_pool.clone() {
                                                    let _ = pool.stop_server(&name).await;
                                                }
                                                let _ = svc.delete_mcp_server(&name);
                                                app.status_message =
                                                    format!("Removed MCP '{}' from pool", name);
                                            }
                                        }

                                        if let Ok(mcp_list) = svc.list_mcp_servers() {
                                            app.mcp_servers = mcp_list;
                                        }
                                        app.input_mode = InputMode::Normal;
                                        app.target_mcp_name = None;
                                    }

                                    InputMode::NewGroupTitle => {
                                        app.input_mode = InputMode::NewGroupCategory;
                                    }
                                    InputMode::SettingsEditor => {
                                        app.config.editor = app.rename_buffer.clone();
                                        std::env::set_var("EDITOR", &app.config.editor);
                                        let _ = app.config.save();
                                        app.input_mode = InputMode::Normal;
                                        app.status_message =
                                            format!("Editor set to '{}'", app.config.editor);
                                    }
                                    InputMode::SettingsInstallPath => {
                                        app.config.install_path = app.rename_buffer.clone();
                                        let _ = app.config.save();
                                        app.input_mode = InputMode::Normal;
                                        app.status_message = format!(
                                            "Install path set to '{}'",
                                            app.config.install_path
                                        );
                                    }
                                    InputMode::SettingsMenu => {
                                        let Some(kind) = app.settings_menu_kind else {
                                            app.input_mode = InputMode::Normal;
                                            continue;
                                        };
                                        let idx = app.settings_menu_state.selected().unwrap_or(0);
                                        let Some((id, _label)) =
                                            app.settings_menu_items.get(idx).cloned()
                                        else {
                                            app.input_mode = InputMode::Normal;
                                            continue;
                                        };

                                        match kind {
                                            SettingsMenuKind::Editor => {
                                                if id == "custom" {
                                                    app.rename_buffer = app.config.editor.clone();
                                                    app.input_mode = InputMode::SettingsEditor;
                                                    continue;
                                                }
                                                app.config.editor = id.clone();
                                                std::env::set_var("EDITOR", &app.config.editor);
                                                let _ = app.config.save();
                                                app.status_message = format!(
                                                    "Editor set to '{}'",
                                                    app.config.editor
                                                );
                                            }
                                            SettingsMenuKind::Theme => {
                                                app.config.theme = id.clone();
                                                let _ = app.config.save();
                                                app.status_message =
                                                    format!("Theme set to '{}'", app.config.theme);
                                            }
                                        }

                                        app.settings_menu_kind = None;
                                        app.settings_menu_items.clear();
                                        app.settings_menu_state.select(Some(0));
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::LspInstaller => {
                                        let available_lsps = crate::tabs::lsp_registry::get_available_lsps();
                                        if let Some(selected_lsp) = available_lsps.get(app.lsp_installer.selected_index) {
                                            let install_cmd = crate::tabs::lsp_registry::get_install_command(selected_lsp);
                                            
                                            app.status_message = format!(
                                                "Installing {}... This may take a few minutes.",
                                                selected_lsp.display_name
                                            );
                                            let _ = terminal.draw(|frame| ui(frame, &mut app));
                                            
                                            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
                                            let exec_result = std::process::Command::new(&shell)
                                                .arg("-c")
                                                .arg(&install_cmd)
                                                .status();
                                            
                                            match exec_result {
                                                Ok(exit_status) => {
                                                    if exit_status.success() {
                                                        app.status_message = format!(
                                                            "{} installed successfully!",
                                                            selected_lsp.display_name
                                                        );
                                                    } else {
                                                        app.status_message = format!(
                                                            "Installation of {} failed with exit code: {:?}",
                                                            selected_lsp.display_name,
                                                            exit_status.code()
                                                        );
                                                    }
                                                }
                                                Err(e) => {
                                                    app.status_message = format!(
                                                        "Failed to start install: {}",
                                                        e
                                                    );
                                                }
                                            }
                                            app.lsp_installer.is_open = false;
                                            app.input_mode = InputMode::Normal;
                                            app.check_lsp_availability();
                                        }
                                    }
                                    InputMode::DiagnosticView => {
                                        app.diagnostic_view.is_open = false;
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::NewGroupCategory => {
                                        if let Some(svc) = service.as_ref() {
                                            let clean_name = app.rename_buffer.trim();
                                            if !clean_name.is_empty() {
                                                let path = format!(
                                                    "/{}",
                                                    clean_name.to_lowercase().replace(' ', "_")
                                                );
                                                let category =
                                                    if app.new_group_category.trim().is_empty() {
                                                        None
                                                    } else {
                                                        Some(app.new_group_category.clone())
                                                    };

                                                // Ensure group exists
                                                let group =
                                                    leindex_core::memory::models::SessionGroup {
                                                        id: 0,
                                                        name: clean_name.to_string(),
                                                        path: path.clone(),
                                                        category: category,
                                                        is_expanded: true,
                                                        sort_order: 0,
                                                        parent_id: None,
                                                    };
                                                let _ = svc.get_or_create_session_group(group);
                                                app.status_message =
                                                    format!("Group '{}' ready", clean_name);
                                                if let Ok(groups) = svc.list_session_groups() {
                                                    app.groups = groups;
                                                }
                                                app.refresh_session_entries();
                                                app.refresh_dash_session_entries();
                                            }
                                        }
                                        app.rename_buffer.clear();
                                        app.new_group_category.clear();
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::MoveToGroup => {
                                        if let Some(svc) = service.as_ref() {
                                            if let Some(id) = app.target_session_id.take() {
                                                let target = if app.rename_buffer.is_empty() {
                                                    None
                                                } else {
                                                    // Ensure the group exists before moving the session
                                                    let clean_name = app.rename_buffer.trim();
                                                    let path = format!(
                                                        "/{}",
                                                        clean_name.to_lowercase().replace(' ', "_")
                                                    );
                                                    let group =
                                                        leindex_core::memory::models::SessionGroup {
                                                            id: 0,
                                                            name: clean_name.to_string(),
                                                            path: path.clone(),
                                                            category: None,
                                                            is_expanded: true,
                                                            sort_order: 0,
                                                            parent_id: None,
                                                        };
                                                    // Create the group if it doesn't exist
                                                    let _ = svc.get_or_create_session_group(group);
                                                    Some(path)
                                                };

                                                let _ = svc.update_session_group(&id, target);
                                                // Refresh groups to include any newly created groups
                                                if let Ok(groups) = svc.list_session_groups() {
                                                    app.groups = groups;
                                                }
                                                if let Ok(sessions) = svc.list_sessions() {
                                                    app.sessions = sessions;
                                                }
                                                app.status_message =
                                                    format!("Session moved to group");
                                                app.refresh_session_entries();
                                                app.refresh_dash_session_entries();
                                            }
                                        }
                                        app.rename_buffer.clear();
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::SessionHub => {
                                        match app.hub_focus {
                                            HubFocus::Rename => {
                                                if let Some(svc) = service.as_ref() {
                                                    if let Some(id) = app.target_session_id.clone()
                                                    {
                                                        let manager = match leindex_core::memory::session_manager::SessionManager::new(svc.clone()) {
                                                            Ok(m) => m,
                                                            Err(e) => {
                                                                app.status_message = format!("Failed to create session manager: {}", e);
                                                                app.input_mode = InputMode::Normal;
                                                                continue;
                                                            }
                                                        };
                                                        let _ = manager.rename_session(
                                                            &id,
                                                            &app.rename_buffer,
                                                        );
                                                        if let Ok(sessions) = svc.list_sessions() {
                                                            app.sessions = sessions;
                                                        }
                                                        app.refresh_session_entries();
                                                        app.refresh_dash_session_entries();
                                                        app.status_message =
                                                            "Session renamed".to_string();
                                                    }
                                                }
                                                app.input_mode = InputMode::Normal;
                                            }
                                            HubFocus::Group => {
                                                // Jump to MoveToGroup logic
                                                app.input_mode = InputMode::MoveToGroup;
                                                app.rename_buffer.clear();
                                            }
                                            HubFocus::Search => {
                                                // Just lose focus or stay? Enter usually commits.
                                                // User says it "terminates the session".
                                                // I'll make it just return to normal for now to avoid accidental kills.
                                                app.input_mode = InputMode::Normal;
                                            }
                                        }
                                    }

                                    InputMode::NewProjectName => {
                                        app.input_mode = InputMode::NewProjectPath;
                                    }
                                    InputMode::NewProjectPath => {
                                        app.input_mode = InputMode::NewProjectTool;
                                    }
                                    InputMode::NewProjectTool => {
                                        // Commit New Project
                                        let name = app.new_project_name.clone();
                                        let path = app.new_project_path.clone();
                                        let tool = app.new_project_tool.clone();
                                        // Execute /maestro:setup via terminal or service
                                        app.status_message = format!(
                                            "Initializing project '{}' at {} with tool {}...",
                                            name, path, tool
                                        );
                                        // In a real impl, we'd spawn a background task for /maestro:setup
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::NewTrackTitle => {
                                        app.input_mode = InputMode::NewTrackType;
                                    }
                                    InputMode::NewTrackType => {
                                        // Commit New Track
                                        let title = app.new_track_title.clone();
                                        let is_master = app.new_track_is_master;
                                        app.status_message = format!(
                                            "Creating {} track: {}...",
                                            if is_master { "master" } else { "direct" },
                                            title
                                        );
                                        // Execute /maestro:newTrack
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::McpLogs => {
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::Normal => {}
                                }
                            }
                            KeyCode::Esc => {
                                match app.input_mode {
                                    InputMode::NewGroupTitle
                                    | InputMode::NewGroupCategory
                                    | InputMode::RenameGroup
                                    | InputMode::RenameGroupCategory => {
                                        app.rename_buffer.clear();
                                        app.new_group_category.clear();
                                        app.target_group_path = None;
                                    }
                                    InputMode::McpLogs => {
                                        app.mcp_log_lines.clear();
                                        app.mcp_log_scroll = 0;
                                        app.target_mcp_name = None;
                                        app.lsp_log_content.clear();
                                        app.lsp_log_scroll = 0;
                                        app.lsp_log_source = None;
                                    }
                                    InputMode::SettingsMenu => {
                                        app.settings_menu_kind = None;
                                        app.settings_menu_items.clear();
                                        app.settings_menu_state.select(Some(0));
                                    }
                                    InputMode::LspInstaller => {
                                        app.lsp_installer.is_open = false;
                                    }
                                    InputMode::DiagnosticView => {
                                        app.diagnostic_view.is_open = false;
                                    }
                                    InputMode::NewMemoryContent | InputMode::NewMemoryCategory => {
                                        app.new_memory_content.clear();
                                        app.new_memory_category.clear();
                                        app.status_message = "Memory creation cancelled".to_string();
                                    }
                                    _ => {}
                                }
                                app.input_mode = InputMode::Normal;
                            }
                            KeyCode::Backspace => match app.input_mode {
                                InputMode::NewSessionTitle => {
                                    app.new_session_title.pop();
                                }
                                InputMode::NewSessionPath => {
                                    app.new_session_path.pop();
                                }
                                InputMode::RenameGroup
                                | InputMode::ForkSession
                                | InputMode::NewGroupTitle
                                | InputMode::MoveToGroup
                                | InputMode::SettingsEditor
                                | InputMode::SettingsInstallPath => {
                                    app.rename_buffer.pop();
                                }
                                InputMode::RenameGroupCategory | InputMode::NewGroupCategory => {
                                    app.new_group_category.pop();
                                }
                                InputMode::KillConfirm | InputMode::DeleteConfirm => {
                                    app.target_session_id = None;
                                    app.input_mode = InputMode::Normal;
                                }
                                InputMode::AnalysisPrompt => {
                                    app.analysis_input.pop();
                                }
                                InputMode::MemorySearch => {
                                    app.memory_query.pop();
                                }
                                InputMode::NewMemoryContent => {
                                    app.new_memory_content.pop();
                                }
                                InputMode::NewMemoryCategory => {
                                    app.new_memory_category.pop();
                                }
                                InputMode::NewProjectName => {
                                    app.new_project_name.pop();
                                }
                                InputMode::NewProjectPath => {
                                    app.new_project_path.pop();
                                }
                                InputMode::NewProjectTool => {
                                    app.new_project_tool.pop();
                                }
                                InputMode::NewTrackTitle => {
                                    app.new_track_title.pop();
                                }
                                InputMode::SessionHub => match app.hub_focus {
                                    HubFocus::Rename => {
                                        app.rename_buffer.pop();
                                    }
                                    HubFocus::Search => {
                                        app.hub_search_buffer.pop();
                                    }
                                    _ => {}
                                },
                                _ => {}
                            },
                            KeyCode::Char(c) => {
                                if c == '\n' || c == '\r' {
                                    continue;
                                }
                                match app.input_mode {
                                    InputMode::NewSessionTitle => app.new_session_title.push(c),
                                    InputMode::NewSessionPath => app.new_session_path.push(c),
                                    InputMode::NewSessionTool => {
                                        // Cycle tools
                                        let tools = vec![
                                            "claude", "gemini", "shell", "codex", "opencode", "amp",
                                        ];
                                        if let Some(pos) =
                                            tools.iter().position(|&t| t == app.new_session_tool)
                                        {
                                            app.new_session_tool =
                                                tools[(pos + 1) % tools.len()].to_string();
                                        }
                                    }
                                    InputMode::RenameGroup
                                    | InputMode::ForkSession
                                    | InputMode::NewGroupTitle
                                    | InputMode::MoveToGroup
                                    | InputMode::SettingsEditor
                                    | InputMode::SettingsInstallPath => app.rename_buffer.push(c),
                                    InputMode::RenameGroupCategory
                                    | InputMode::NewGroupCategory => app.new_group_category.push(c),
                                    InputMode::NewProjectName => app.new_project_name.push(c),
                                    InputMode::NewProjectPath => app.new_project_path.push(c),
                                    InputMode::NewProjectTool => app.new_project_tool.push(c),
                                    InputMode::NewTrackTitle => app.new_track_title.push(c),
                                    InputMode::NewTrackType => {
                                        if c == ' ' {
                                            app.new_track_is_master = !app.new_track_is_master;
                                        }
                                    }
                                    InputMode::SessionHub => {
                                        match app.hub_focus {
                                            HubFocus::Rename => app.rename_buffer.push(c),
                                            HubFocus::Search => {
                                                app.hub_search_buffer.push(c);
                                                // Trigger live search logic here if needed
                                            }
                                            _ => {}
                                        }
                                    }
                                    InputMode::MemorySearch => {
                                        app.memory_query.push(c);
                                    }
                                    InputMode::NewMemoryContent => {
                                        app.new_memory_content.push(c);
                                    }
                                    InputMode::NewMemoryCategory => {
                                        app.new_memory_category.push(c);
                                    }
                                    InputMode::KillConfirm | InputMode::DeleteConfirm => {
                                        if c == 'y' || c == 'Y' {
                                            if let Some(svc) = service.as_ref() {
                                                if let Some(id) = app.target_session_id.take() {
                                                    let manager = match leindex_core::memory::session_manager::SessionManager::new(svc.clone()) {
                                                        Ok(m) => m,
                                                        Err(e) => {
                                                            app.status_message = format!("Failed to create session manager: {}", e);
                                                            continue;
                                                        }
                                                    };
                                                    match manager.kill_session(&id) {
                                                        Ok(()) => {
                                                            if app.input_mode
                                                                == InputMode::DeleteConfirm
                                                            {
                                                                let _ = svc.delete_session(&id);
                                                                app.status_message =
                                                                    "Session deleted".to_string();
                                                            } else {
                                                                app.status_message =
                                                                    "Session killed".to_string();
                                                            }
                                                            if let Ok(sessions) =
                                                                svc.list_sessions()
                                                            {
                                                                app.sessions = sessions;
                                                            }
                                                        }
                                                        Err(e) => {
                                                            app.status_message =
                                                                format!("Kill failed: {}", e);
                                                        }
                                                    }
                                                }
                                                // Group delete logic
                                                if let Some(path) = app.target_group_path.take() {
                                                    let _ = svc.delete_group(&path);
                                                    app.status_message = format!("Group deleted");
                                                    if let Ok(groups) = svc.list_session_groups() {
                                                        app.groups = groups;
                                                    }
                                                    if let Ok(sessions) = svc.list_sessions() {
                                                        app.sessions = sessions;
                                                    }
                                                }
                                            }
                                        }
                                        app.refresh_session_entries();
                                        app.refresh_dash_session_entries();
                                        app.target_session_id = None;
                                        app.target_group_path = None;
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::AnalysisPrompt => {
                                        app.analysis_input.push(c);
                                    }
                                    _ => {}
                                }
                            }
                            KeyCode::Tab => match app.input_mode {
                                InputMode::NewGroupTitle => {
                                    app.input_mode = InputMode::NewGroupCategory;
                                }
                                InputMode::NewGroupCategory => {
                                    app.input_mode = InputMode::NewGroupTitle;
                                }
                                InputMode::RenameGroup => {
                                    app.input_mode = InputMode::RenameGroupCategory;
                                }
                                InputMode::RenameGroupCategory => {
                                    app.input_mode = InputMode::RenameGroup;
                                }
                                InputMode::SessionHub => {
                                    app.hub_focus = match app.hub_focus {
                                        HubFocus::Rename => HubFocus::Group,
                                        HubFocus::Group => HubFocus::Search,
                                        HubFocus::Search => HubFocus::Rename,
                                    };
                                }
                                InputMode::Normal if app.tab_index == 0 => {
                                    app.dash_focus = match app.dash_focus {
                                        DashFocus::Sessions => DashFocus::Mcp,
                                        DashFocus::Mcp => DashFocus::Tabs,
                                        DashFocus::Tabs => DashFocus::Sessions,
                                    };
                                }
                                InputMode::Normal if app.tab_index == 4 => {
                                    app.conductor.output_focused = !app.conductor.output_focused;
                                }
                                _ => {}
                            },
                            KeyCode::BackTab => match app.input_mode {
                                InputMode::NewGroupTitle => {
                                    app.input_mode = InputMode::NewGroupCategory;
                                }
                                InputMode::NewGroupCategory => {
                                    app.input_mode = InputMode::NewGroupTitle;
                                }
                                InputMode::RenameGroup => {
                                    app.input_mode = InputMode::RenameGroupCategory;
                                }
                                InputMode::RenameGroupCategory => {
                                    app.input_mode = InputMode::RenameGroup;
                                }
                                InputMode::SessionHub => {
                                    app.hub_focus = match app.hub_focus {
                                        HubFocus::Rename => HubFocus::Search,
                                        HubFocus::Group => HubFocus::Rename,
                                        HubFocus::Search => HubFocus::Group,
                                    };
                                }
                                InputMode::Normal if app.tab_index == 0 => {
                                    app.dash_focus = match app.dash_focus {
                                        DashFocus::Sessions => DashFocus::Tabs,
                                        DashFocus::Mcp => DashFocus::Sessions,
                                        DashFocus::Tabs => DashFocus::Mcp,
                                    };
                                }
                                InputMode::Normal if app.tab_index == 4 => {
                                    app.conductor.output_focused = !app.conductor.output_focused;
                                }
                                _ => {}
                            },
                            KeyCode::Down if app.tab_index != 4 => {
                                if app.input_mode == InputMode::SessionSwitcher {
                                    let i = match app.switcher_state.selected() {
                                        Some(i) => {
                                            if i >= app.sessions.len().saturating_sub(1) {
                                                0
                                            } else {
                                                i + 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    app.switcher_state.select(Some(i));
                                } else if app.input_mode == InputMode::McpLogs {
                                    if app.lsp_log_source.is_some() {
                                        app.lsp_log_scroll = app.lsp_log_scroll.saturating_add(1);
                                    } else {
                                        app.mcp_log_scroll = app.mcp_log_scroll.saturating_add(1);
                                    }
                                } else if app.input_mode == InputMode::SettingsMenu {
                                    let len = app.settings_menu_items.len();
                                    let i = match app.settings_menu_state.selected() {
                                        Some(i) => {
                                            if i >= len.saturating_sub(1) {
                                                0
                                            } else {
                                                i + 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    app.settings_menu_state.select(Some(i));
                                } else if app.input_mode == InputMode::McpMenu {
                                    app.mcp_menu_option = match app.mcp_menu_option {
                                        McpOption::Start => McpOption::Stop,
                                        McpOption::Stop => McpOption::Pause,
                                        McpOption::Pause => McpOption::Logs,
                                        McpOption::Logs => McpOption::Add,
                                        McpOption::Add => McpOption::Remove,
                                        McpOption::Remove => McpOption::Start,
                                    };
                                } else if app.input_mode == InputMode::LspInstaller {
                                    let count = crate::tabs::lsp_registry::get_available_lsps().len();
                                    if app.lsp_installer.selected_index < count.saturating_sub(1) {
                                        app.lsp_installer.selected_index += 1;
                                    } else {
                                        app.lsp_installer.selected_index = 0;
                                    }
                                } else if app.input_mode == InputMode::DiagnosticView {
                                    let count = app.lsp_diagnostics_cache.len();
                                    if count > 0 && app.diagnostic_view.selected_index < count.saturating_sub(1) {
                                        app.diagnostic_view.selected_index += 1;
                                    } else {
                                        app.diagnostic_view.selected_index = 0;
                                    }
                                } else if app.preview_focused {
                                    app.preview_scroll = app.preview_scroll.saturating_add(1);
                                } else if app.tab_index == 0 {
                                    // Dashboard
                                    match app.dash_focus {
                                        DashFocus::Sessions => {
                                            app.dash_select_next_session();
                                        }
                                        DashFocus::Mcp => {
                                            let i = match app.mcp_state.selected() {
                                                Some(i) => {
                                                    if i >= app.mcp_servers.len().saturating_sub(1)
                                                    {
                                                        0
                                                    } else {
                                                        i + 1
                                                    }
                                                }
                                                None => 0,
                                            };
                                            app.mcp_state.select(Some(i));
                                        }
                                        DashFocus::Tabs => {}
                                    }
                                } else if app.tab_index == 2 {
                                    // Projects
                                    if app.preview_focused {
                                        app.project_explorer_selected =
                                            (app.project_explorer_selected + 1)
                                                % app.explorer_items.len().max(1);
                                    } else {
                                        let i = match app.project_state.selected() {
                                            Some(i) => {
                                                if i >= app.projects.len().saturating_sub(1) {
                                                    0
                                                } else {
                                                    i + 1
                                                }
                                            }
                                            None => 0,
                                        };
                                        app.project_state.select(Some(i));
                                        app.project_explorer_path = None;
                                        app.project_explorer_selected = 0;
                                    }
                                } else if app.tab_index == 1 {
                                    // Sessions Tab
                                    let i = match app.session_state.selected() {
                                        Some(i) => {
                                            if i >= app.session_entries.len().saturating_sub(1) {
                                                0
                                            } else {
                                                i + 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    app.session_state.select(Some(i));
                                } else if app.tab_index == 5 {
                                    // Memory
                                    let i = match app.memory_state.selected() {
                                        Some(i) => {
                                            if i >= app.memories.len().saturating_sub(1) {
                                                0
                                            } else {
                                                i + 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    app.memory_state.select(Some(i));
                                } else if app.tab_index == 8 {
                                    // Settings
                                    app.settings_option = match app.settings_option {
                                        SettingsOption::Editor => SettingsOption::Theme,
                                        SettingsOption::Theme => SettingsOption::Transparent,
                                        SettingsOption::Transparent => SettingsOption::InstallPath,
                                        SettingsOption::InstallPath => SettingsOption::Save,
                                        SettingsOption::Save => SettingsOption::Editor,
                                    };
                                }
                                app.scroll = app.scroll.saturating_add(1);
                            }
                            KeyCode::Up if app.tab_index != 4 => {
                                if app.input_mode == InputMode::McpMenu {
                                    app.mcp_menu_option = match app.mcp_menu_option {
                                        McpOption::Start => McpOption::Remove,
                                        McpOption::Stop => McpOption::Start,
                                        McpOption::Pause => McpOption::Stop,
                                        McpOption::Logs => McpOption::Pause,
                                        McpOption::Add => McpOption::Logs,
                                        McpOption::Remove => McpOption::Add,
                                    };
                                } else if app.input_mode == InputMode::McpLogs {
                                    if app.lsp_log_source.is_some() {
                                        app.lsp_log_scroll = app.lsp_log_scroll.saturating_sub(1);
                                    } else {
                                        app.mcp_log_scroll = app.mcp_log_scroll.saturating_sub(1);
                                    }
                                } else if app.input_mode == InputMode::SettingsMenu {
                                    let len = app.settings_menu_items.len();
                                    let i = match app.settings_menu_state.selected() {
                                        Some(i) => {
                                            if i == 0 {
                                                len.saturating_sub(1)
                                            } else {
                                                i - 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    app.settings_menu_state.select(Some(i));
                                } else if app.input_mode == InputMode::LspInstaller {
                                    let count = crate::tabs::lsp_registry::get_available_lsps().len();
                                    if app.lsp_installer.selected_index > 0 {
                                        app.lsp_installer.selected_index -= 1;
                                    } else {
                                        app.lsp_installer.selected_index = count.saturating_sub(1);
                                    }
                                } else if app.input_mode == InputMode::DiagnosticView {
                                    let count = app.lsp_diagnostics_cache.len();
                                    if count > 0 && app.diagnostic_view.selected_index > 0 {
                                        app.diagnostic_view.selected_index -= 1;
                                    } else if count > 0 {
                                        app.diagnostic_view.selected_index = count.saturating_sub(1);
                                    }
                                } else if app.preview_focused {
                                    app.preview_scroll = app.preview_scroll.saturating_sub(1);
                                } else if app.tab_index == 0 {
                                    // Dashboard
                                    match app.dash_focus {
                                        DashFocus::Sessions => {
                                            app.dash_select_prev_session();
                                        }
                                        DashFocus::Mcp => {
                                            let i = match app.mcp_state.selected() {
                                                Some(i) => {
                                                    if i == 0 {
                                                        app.mcp_servers.len().saturating_sub(1)
                                                    } else {
                                                        i - 1
                                                    }
                                                }
                                                None => 0,
                                            };
                                            app.mcp_state.select(Some(i));
                                        }
                                        DashFocus::Tabs => {}
                                    }
                                } else if app.tab_index == 2 {
                                    // Projects Tab
                                    if app.preview_focused {
                                        app.project_explorer_selected =
                                            if app.project_explorer_selected == 0 {
                                                app.explorer_items.len().saturating_sub(1)
                                            } else {
                                                app.project_explorer_selected - 1
                                            };
                                    } else {
                                        let i = match app.project_state.selected() {
                                            Some(i) => {
                                                if i == 0 {
                                                    app.projects.len().saturating_sub(1)
                                                } else {
                                                    i - 1
                                                }
                                            }
                                            None => 0,
                                        };
                                        app.project_state.select(Some(i));
                                        app.project_explorer_path = None;
                                        app.project_explorer_selected = 0;
                                    }
                                } else if app.tab_index == 1 {
                                    // Sessions Tab
                                    let i = match app.session_state.selected() {
                                        Some(i) => {
                                            if i == 0 {
                                                app.session_entries.len().saturating_sub(1)
                                            } else {
                                                i - 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    app.session_state.select(Some(i));
                                } else if app.tab_index == 5 {
                                    // Memory
                                    let i = match app.memory_state.selected() {
                                        Some(i) => {
                                            if i == 0 {
                                                app.memories.len().saturating_sub(1)
                                            } else {
                                                i - 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    app.memory_state.select(Some(i));
                                } else if app.tab_index == 8 {
                                    // Settings
                                    app.settings_option = match app.settings_option {
                                        SettingsOption::Editor => SettingsOption::Save,
                                        SettingsOption::Save => SettingsOption::InstallPath,
                                        SettingsOption::InstallPath => SettingsOption::Transparent,
                                        SettingsOption::Transparent => SettingsOption::Theme,
                                        SettingsOption::Theme => SettingsOption::Editor,
                                    };
                                }
                                app.scroll = app.scroll.saturating_sub(1);
                            }
                            _ => {}
                        }
                    } else if app.show_help {
                        let max_scroll =
                            modals::build_help_text(&app).len().saturating_sub(1) as u16;

                        match key.code {
                            KeyCode::Esc | KeyCode::Char('/') | KeyCode::Char('?') => {
                                app.show_help = false;
                            }
                            KeyCode::Up => app.help_scroll = app.help_scroll.saturating_sub(1),
                            KeyCode::Down => app.help_scroll = app.help_scroll.saturating_add(1),
                            KeyCode::PageUp => app.help_scroll = app.help_scroll.saturating_sub(10),
                            KeyCode::PageDown => {
                                app.help_scroll = app.help_scroll.saturating_add(10)
                            }
                            KeyCode::Home => app.help_scroll = 0,
                            KeyCode::End => app.help_scroll = max_scroll,
                            _ => {}
                        }
                        app.help_scroll = app.help_scroll.min(max_scroll);
                    } else {
                        match (key.modifiers, key.code) {
                            // 1. Global Navigation (Highest Priority)
                            // Tab / BackTab
                            (KeyModifiers::NONE, KeyCode::Tab) => {
                                if app.tab_index == 0 {
                                    match app.dash_focus {
                                        DashFocus::Sessions => app.dash_focus = DashFocus::Mcp,
                                        DashFocus::Mcp => app.dash_focus = DashFocus::Tabs,
                                        DashFocus::Tabs => {
                                            app.tab_index = 1;
                                            app.dash_focus = DashFocus::Sessions;
                                        }
                                    };
                                } else {
                                    app.tab_index = (app.tab_index + 1) % 9; // 9 tabs now (including Settings)
                                    app.preview_focused = false;
                                }
                            }
                            (KeyModifiers::SHIFT, KeyCode::BackTab) | (_, KeyCode::BackTab) if app.tab_index != 6 => {
                                if app.tab_index == 0 {
                                    match app.dash_focus {
                                        DashFocus::Sessions => {
                                            app.tab_index = 7;  // Now Settings (was 6)
                                            app.dash_focus = DashFocus::Sessions;
                                        }
                                        DashFocus::Mcp => app.dash_focus = DashFocus::Sessions,
                                        DashFocus::Tabs => app.dash_focus = DashFocus::Mcp,
                                    };
                                } else {
                                    app.tab_index = if app.tab_index == 0 {
                                        8  // Wrap to last tab (Settings)
                                    } else {
                                        app.tab_index - 1
                                    };
                                    app.preview_focused = false;
                                }
                            }

                            // 2. Direct Numbers (Global)
                            (KeyModifiers::NONE, KeyCode::Char('1')) => app.tab_index = 0,
                            (KeyModifiers::NONE, KeyCode::Char('2')) => app.tab_index = 1,
                            (KeyModifiers::NONE, KeyCode::Char('3')) => app.tab_index = 2,
                            (KeyModifiers::NONE, KeyCode::Char('4')) => app.tab_index = 3,
                            (KeyModifiers::NONE, KeyCode::Char('5')) => app.tab_index = 4,
                            (KeyModifiers::NONE, KeyCode::Char('6')) => app.tab_index = 5,
                            (KeyModifiers::NONE, KeyCode::Char('7')) => app.tab_index = 6,
                            (KeyModifiers::NONE, KeyCode::Char('8')) => app.tab_index = 7,
                            (KeyModifiers::NONE, KeyCode::Char('9')) => app.tab_index = 8,

                            // 3. Conductor Specific Alt Keys
                            (KeyModifiers::ALT, KeyCode::Char('1')) if app.tab_index == 4 => {
                                app.conductor.details_mode =
                                    crate::conductor::model::DetailsViewMode::Details;
                            }
                            (KeyModifiers::ALT, KeyCode::Char('2')) if app.tab_index == 4 => {
                                app.conductor.details_mode =
                                    crate::conductor::model::DetailsViewMode::Output;
                            }
                            (KeyModifiers::ALT, KeyCode::Char('3')) if app.tab_index == 4 => {
                                app.conductor.details_mode =
                                    crate::conductor::model::DetailsViewMode::Prompt;
                            }
                            (KeyModifiers::ALT, KeyCode::Char('p')) if app.tab_index == 4 => {
                                app.conductor.output_focused = !app.conductor.output_focused;
                                app.status_message = if app.conductor.output_focused {
                                    "Output focused. Scroll with Arrows/PgUp/PgDn."
                                } else {
                                    "Tracks focused."
                                }
                                .to_string();
                            }

                            // 4. Global Alt Keys (Tab Switching)
                            (KeyModifiers::ALT, KeyCode::Char('1')) => app.tab_index = 0,
                            (KeyModifiers::ALT, KeyCode::Char('2')) => app.tab_index = 1,
                            (KeyModifiers::ALT, KeyCode::Char('3')) => app.tab_index = 2,
                            (KeyModifiers::ALT, KeyCode::Char('4')) => app.tab_index = 3,
                            (KeyModifiers::ALT, KeyCode::Char('5')) => app.tab_index = 4,
                            (KeyModifiers::ALT, KeyCode::Char('6')) => app.tab_index = 5,
                            (KeyModifiers::ALT, KeyCode::Char('7')) => app.tab_index = 6,
                            (KeyModifiers::ALT, KeyCode::Char('8')) => app.tab_index = 7,
                            (KeyModifiers::ALT, KeyCode::Char('9')) => app.tab_index = 8,

                            // 5. Conductor Catch-All
                            _ if app.tab_index == 4 => {
                                use crate::conductor::keybindings::{
                                    handle_key_event, ConductorAction,
                                };
                                match handle_key_event(&mut app.conductor, key) {
                                    ConductorAction::Handled => continue,
                                    ConductorAction::StatusMessage(msg) => {
                                        app.toast_queue.info(msg);
                                        continue;
                                    }
                                    ConductorAction::StoreMemory { content, category } => {
                                        if let Some(svc) = service.as_ref() {
                                            match svc.store_memory(&content, category) {
                                                Ok(id) => {
                                                    app.toast_queue.success(format!("Memory stored with ID {}", id));
                                                    app.refresh_from_service(&service);
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!("Failed to store memory: {}", e));
                                                }
                                            }
                                        } else {
                                            app.toast_queue.error("Memory service not available");
                                        }
                                        continue;
                                    }
                                    ConductorAction::DeleteMemory { id } => {
                                        if let Some(svc) = service.as_ref() {
                                            match svc.delete_memory(id) {
                                                Ok(true) => {
                                                    app.toast_queue.success(format!("Memory {} deleted", id));
                                                    app.refresh_from_service(&service);
                                                }
                                                Ok(false) => {
                                                    app.toast_queue.warning(format!("Memory {} not found", id));
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!("Failed to delete memory: {}", e));
                                                }
                                            }
                                        } else {
                                            app.toast_queue.error("Memory service not available");
                                        }
                                        continue;
                                    }
                                    ConductorAction::None => {}
                                }
                                // Fall through for global keys like 'q'
                            }

                            // 5b. Ralph Loop Keys for Conductor (explicit handling for s, p, r, ?)
                            (KeyModifiers::NONE, KeyCode::Char('s')) if app.tab_index == 4 => {
                                // Start/Run track
                                use crate::conductor::keybindings::{
                                    handle_key_event, ConductorAction,
                                };
                                match handle_key_event(&mut app.conductor, key) {
                                    ConductorAction::Handled => continue,
                                    ConductorAction::StatusMessage(msg) => {
                                        app.toast_queue.info(msg);
                                        continue;
                                    }
                                    ConductorAction::StoreMemory { content, category } => {
                                        if let Some(svc) = service.as_ref() {
                                            match svc.store_memory(&content, category) {
                                                Ok(id) => {
                                                    app.toast_queue.success(format!("Memory stored with ID {}", id));
                                                    app.refresh_from_service(&service);
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!("Failed to store memory: {}", e));
                                                }
                                            }
                                        } else {
                                            app.toast_queue.error("Memory service not available");
                                        }
                                        continue;
                                    }
                                                                        ConductorAction::DeleteMemory { id } => {
                                                                            if let Some(svc) = service.as_ref() {
                                                                                match svc.delete_memory(id) {
                                                                                    Ok(true) => {
                                                                                        app.toast_queue.success(format!("Memory {} deleted", id));
                                                                                        app.refresh_from_service(&service);
                                                                                    }
                                                                                    Ok(false) => {
                                                                                        app.toast_queue.warning(format!("Memory {} not found", id));
                                                                                    }
                                                                                    Err(e) => {
                                                                                        app.toast_queue.error(format!("Failed to delete memory: {}", e));
                                                                                    }
                                                                                }
                                                                            } else {
                                                                                app.toast_queue.error("Memory service not available");
                                                                            }
                                                                            continue;
                                                                        }
                                                                        ConductorAction::None => {}
                                                                    }
                                                                }
                                    
                                                                (KeyModifiers::NONE, KeyCode::Char('p')) if app.tab_index == 4 => {
                                // Pause track
                                use crate::conductor::keybindings::{
                                    handle_key_event, ConductorAction,
                                };
                                match handle_key_event(&mut app.conductor, key) {
                                    ConductorAction::Handled => continue,
                                    ConductorAction::StatusMessage(msg) => {
                                        app.toast_queue.info(msg);
                                        continue;
                                    }
                                    ConductorAction::StoreMemory { content, category } => {
                                        if let Some(svc) = service.as_ref() {
                                            match svc.store_memory(&content, category) {
                                                Ok(id) => {
                                                    app.toast_queue.success(format!("Memory stored with ID {}", id));
                                                    app.refresh_from_service(&service);
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!("Failed to store memory: {}", e));
                                                }
                                            }
                                        } else {
                                            app.toast_queue.error("Memory service not available");
                                        }
                                        continue;
                                    }
                                                                        ConductorAction::DeleteMemory { id } => {
                                                                            if let Some(svc) = service.as_ref() {
                                                                                match svc.delete_memory(id) {
                                                                                    Ok(true) => {
                                                                                        app.toast_queue.success(format!("Memory {} deleted", id));
                                                                                        app.refresh_from_service(&service);
                                                                                    }
                                                                                    Ok(false) => {
                                                                                        app.toast_queue.warning(format!("Memory {} not found", id));
                                                                                    }
                                                                                    Err(e) => {
                                                                                        app.toast_queue.error(format!("Failed to delete memory: {}", e));
                                                                                    }
                                                                                }
                                                                            } else {
                                                                                app.toast_queue.error("Memory service not available");
                                                                            }
                                                                            continue;
                                                                        }
                                                                        ConductorAction::None => {}
                                                                    }
                                                                }
                                    
                                                                (KeyModifiers::NONE, KeyCode::Char('r')) if app.tab_index == 4 => {
                                // Resume track
                                use crate::conductor::keybindings::{
                                    handle_key_event, ConductorAction,
                                };
                                match handle_key_event(&mut app.conductor, key) {
                                    ConductorAction::Handled => continue,
                                    ConductorAction::StatusMessage(msg) => {
                                        app.toast_queue.info(msg);
                                        continue;
                                    }
                                    ConductorAction::StoreMemory { content, category } => {
                                        if let Some(svc) = service.as_ref() {
                                            match svc.store_memory(&content, category) {
                                                Ok(id) => {
                                                    app.toast_queue.success(format!("Memory stored with ID {}", id));
                                                    app.refresh_from_service(&service);
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!("Failed to store memory: {}", e));
                                                }
                                            }
                                        } else {
                                            app.toast_queue.error("Memory service not available");
                                        }
                                        continue;
                                    }
                                                                        ConductorAction::DeleteMemory { id } => {
                                                                            if let Some(svc) = service.as_ref() {
                                                                                match svc.delete_memory(id) {
                                                                                    Ok(true) => {
                                                                                        app.toast_queue.success(format!("Memory {} deleted", id));
                                                                                        app.refresh_from_service(&service);
                                                                                    }
                                                                                    Ok(false) => {
                                                                                        app.toast_queue.warning(format!("Memory {} not found", id));
                                                                                    }
                                                                                    Err(e) => {
                                                                                        app.toast_queue.error(format!("Failed to delete memory: {}", e));
                                                                                    }
                                                                                }
                                                                            } else {
                                                                                app.toast_queue.error("Memory service not available");
                                                                            }
                                                                            continue;
                                                                        }
                                                                        ConductorAction::None => {}
                                                                    }
                                                                }
                                    
                                                                (KeyModifiers::NONE, KeyCode::Char('?')) if app.tab_index == 4 => {
                                // Show status/help
                                use crate::conductor::keybindings::{
                                    handle_key_event, ConductorAction,
                                };
                                match handle_key_event(&mut app.conductor, key) {
                                    ConductorAction::Handled => continue,
                                    ConductorAction::StatusMessage(msg) => {
                                        app.toast_queue.info(msg);
                                        continue;
                                    }
                                    ConductorAction::StoreMemory { content, category } => {
                                        if let Some(svc) = service.as_ref() {
                                            match svc.store_memory(&content, category) {
                                                Ok(id) => {
                                                    app.toast_queue.success(format!("Memory stored with ID {}", id));
                                                    app.refresh_from_service(&service);
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!("Failed to store memory: {}", e));
                                                }
                                            }
                                        } else {
                                            app.toast_queue.error("Memory service not available");
                                        }
                                        continue;
                                    }
                                    ConductorAction::DeleteMemory { id } => {
                                        if let Some(svc) = service.as_ref() {
                                            match svc.delete_memory(id) {
                                                Ok(true) => {
                                                    app.toast_queue.success(format!("Memory {} deleted", id));
                                                    app.refresh_from_service(&service);
                                                }
                                                Ok(false) => {
                                                    app.toast_queue.warning(format!("Memory {} not found", id));
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!("Failed to delete memory: {}", e));
                                                }
                                            }
                                        } else {
                                            app.toast_queue.error("Memory service not available");
                                        }
                                        continue;
                                    }
                                    ConductorAction::None => {}
                                }
                            }

                            (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                                if app.tab_index == 5 {
                                    app.input_mode = InputMode::MemorySearch;
                                }
                            }
                            (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                                if app.tab_index == 5 {
                                    app.memory_query.clear();
                                    app.refresh_from_service(&service);
                                }
                            }
                            (KeyModifiers::NONE, KeyCode::Char('n')) => {
                                // Memory tab: Start new memory creation
                                // Handle both tab 5 and 6 (both are memory tabs)
                                if app.tab_index == 5 || app.tab_index == 6 {
                                    app.new_memory_content.clear();
                                    app.new_memory_category.clear();
                                    app.input_mode = InputMode::NewMemoryContent;
                                    app.status_message = "Creating new memory - enter content".to_string();
                                } else {
                                    // New session wizard for other tabs
                                    app.input_mode = InputMode::NewSessionTitle;
                                    // Auto-fill path if a project is selected
                                    if app.tab_index == 2 {
                                        // Projects Tab
                                        if let Some(i) = app.project_state.selected() {
                                            app.new_session_path = app.projects[i].path.clone();
                                            app.new_session_title =
                                                format!("Chat: {}", app.projects[i].name);
                                        }
                                    }
                                }
                            }
                            (KeyModifiers::ALT, KeyCode::Char('p')) if app.tab_index == 1 => {
                                app.preview_focused = !app.preview_focused;
                                app.status_message = if app.preview_focused {
                                    "Preview focused. Scroll with Arrows/PgUp/PgDn."
                                } else {
                                    "List focused."
                                }
                                .to_string();
                            }
                            (KeyModifiers::ALT, KeyCode::Up)
                            | (KeyModifiers::ALT, KeyCode::Down)
                                if app.tab_index != 4 =>
                            {
                                if app.tab_index == 1 {
                                    let Some(svc) = service.as_ref() else {
                                        app.status_message =
                                            "Error: Memory service not available".to_string();
                                        continue;
                                    };

                                    let delta: i32 = if matches!(key.code, KeyCode::Up) {
                                        -1
                                    } else {
                                        1
                                    };
                                    let Some(selected) = app.session_state.selected() else {
                                        continue;
                                    };
                                    let Some(entry) = app.session_entries.get(selected).cloned()
                                    else {
                                        continue;
                                    };

                                    match entry {
                                        SessionEntry::Group(g) => {
                                            if g.path == "uncategorized" {
                                                app.status_message =
                                                    "Cannot reorder [Uncategorized]".to_string();
                                                continue;
                                            }

                                            let mut paths: Vec<String> = app
                                                .groups
                                                .iter()
                                                .map(|gg| gg.path.clone())
                                                .collect();
                                            let Some(pos) = paths.iter().position(|p| p == &g.path)
                                            else {
                                                continue;
                                            };

                                            let new_pos = if delta < 0 {
                                                pos.checked_sub(1)
                                            } else if pos + 1 < paths.len() {
                                                Some(pos + 1)
                                            } else {
                                                None
                                            };
                                            let Some(new_pos) = new_pos else {
                                                continue;
                                            };

                                            paths.swap(pos, new_pos);
                                            if svc.reorder_session_groups(&paths).is_ok() {
                                                if let Ok(groups) = svc.list_session_groups() {
                                                    app.groups = groups;
                                                }
                                                if let Ok(sessions) = svc.list_sessions() {
                                                    app.sessions = sessions;
                                                }
                                                app.refresh_session_entries();
                                                app.refresh_dash_session_entries();
                                            }
                                        }
                                        SessionEntry::Session(s) => {
                                            let group_key = s.group_path.clone();
                                            let mut ids: Vec<String> = app
                                                .sessions
                                                .iter()
                                                .filter(|ss| ss.group_path == group_key)
                                                .map(|ss| ss.session_id.clone())
                                                .collect();

                                            let Some(pos) =
                                                ids.iter().position(|id| id == &s.session_id)
                                            else {
                                                continue;
                                            };
                                            let new_pos = if delta < 0 {
                                                pos.checked_sub(1)
                                            } else if pos + 1 < ids.len() {
                                                Some(pos + 1)
                                            } else {
                                                None
                                            };
                                            let Some(new_pos) = new_pos else {
                                                continue;
                                            };

                                            ids.swap(pos, new_pos);
                                            if svc
                                                .reorder_sessions_in_group(
                                                    group_key.as_deref(),
                                                    &ids,
                                                )
                                                .is_ok()
                                            {
                                                if let Ok(sessions) = svc.list_sessions() {
                                                    app.sessions = sessions;
                                                }
                                                app.refresh_session_entries();
                                                app.refresh_dash_session_entries();
                                            }
                                        }
                                    }
                                }
                            }
                            (_, KeyCode::Char('p')) if app.tab_index != 4 => {
                                if app.tab_index == 2 {
                                    // Projects
                                    app.input_mode = InputMode::NewProjectName;
                                    app.new_project_name.clear();
                                    app.new_project_path = std::env::current_dir()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string();
                                    app.new_project_tool.clear();
                                }
                            }
                            (_, KeyCode::Char('t')) => {
                                if app.tab_index == 2 {
                                    // Projects
                                    app.input_mode = InputMode::NewTrackTitle;
                                    app.new_track_title.clear();
                                    app.new_track_is_master = true;
                                }
                            }
                            (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
                                // Do nothing, let tmux handle it or avoid accidental TUI quit
                            }
                            (_, KeyCode::Char('q')) => {
                                if app.project_view_open {
                                    app.project_view_open = false;
                                } else if app.show_help {
                                    app.show_help = false;
                                } else {
                                    app.should_quit = true;
                                }
                            }
                            (KeyModifiers::CONTROL, KeyCode::Char('c')) => app.should_quit = true,
                            (_, KeyCode::Esc) if app.tab_index != 4 => {
                                if app.project_view_open {
                                    app.project_view_open = false;
                                }
                            }
                            (_, KeyCode::Char('r')) if app.tab_index != 4 => {
                                if app.tab_index == 1 {
                                    if let Some(i) = app.session_state.selected() {
                                        match app.session_entries.get(i) {
                                            Some(SessionEntry::Session(s)) => {
                                                app.target_session_id = Some(s.session_id.clone());
                                                app.rename_buffer = s.title.clone();
                                                app.hub_search_buffer.clear();
                                                app.input_mode = InputMode::SessionHub;
                                            }
                                            Some(SessionEntry::Group(g)) => {
                                                app.target_group_path = Some(g.path.clone());
                                                app.rename_buffer = g.name.clone();
                                                app.new_group_category =
                                                    g.category.clone().unwrap_or_default();
                                                app.input_mode = InputMode::RenameGroup;
                                            }
                                            _ => {}
                                        }
                                    }
                                } else if app.tab_index == 5 {
                                    if let Some(svc) = service.as_ref() {
                                        match svc.sync_memories_from_system() {
                                            Ok(n) => {
                                                app.status_message = format!(
                                                    "Memory refresh imported {} record(s)",
                                                    n
                                                )
                                            }
                                            Err(e) => {
                                                app.status_message =
                                                    format!("Memory refresh failed: {}", e)
                                            }
                                        }
                                        app.refresh_from_service(&service);
                                    }
                                } else if app.tab_index == 6 {
                                    // Ktop tab - manual refresh
                                    if app.ktop_state.is_some() {
                                        app.ktop_state.as_mut().unwrap().mark_refreshed();
                                        app.status_message = "Krustop refreshed".to_string();
                                    }
                                } else if app.tab_index == 7 {
                                    // LSPs tab
                                    let scheduled = app.queue_lsp_autostart_for_sessions();
                                    // Use force=true for manual refresh to bypass throttle
                                    if app.refresh_lsp_status_impl(true) {
                                        app.status_message = if scheduled {
                                            "LSP scan queued; refreshing status".to_string()
                                        } else {
                                            "LSP status refreshed".to_string()
                                        };
                                    } else {
                                        // Should not happen with force=true, but handle gracefully
                                        app.status_message = "LSP refresh pending...".to_string();
                                    }
                                }
                            }
                            (_, KeyCode::Char('k')) if app.tab_index != 4 => {
                                if app.tab_index == 1 {
                                    if let Some(i) = app.session_state.selected() {
                                        if let Some(SessionEntry::Session(s)) =
                                            app.session_entries.get(i)
                                        {
                                            app.target_session_id = Some(s.session_id.clone());
                                            app.input_mode = InputMode::KillConfirm;
                                        }
                                    }
                                } else if app.tab_index == 0
                                    && app.dash_focus == DashFocus::Sessions
                                {
                                    if let Some(session_id) =
                                        app.dash_selected_session().map(|s| s.session_id.clone())
                                    {
                                        app.target_session_id = Some(session_id);
                                        app.input_mode = InputMode::KillConfirm;
                                    }
                                }
                            }
                            (_, KeyCode::Char('D') | KeyCode::Char('d')) if app.tab_index != 4 && app.tab_index != 6 && app.tab_index != 7 => {
                                if app.tab_index == 1 {
                                    if let Some(i) = app.session_state.selected() {
                                        if let Some(entry) = app.session_entries.get(i) {
                                            match entry {
                                                SessionEntry::Session(s) => {
                                                    app.target_session_id =
                                                        Some(s.session_id.clone());
                                                    app.status_message = format!(
                                                        "Confirm DELETE session '{}'? (y/n)",
                                                        s.title
                                                    );
                                                    app.input_mode = InputMode::DeleteConfirm;
                                                }
                                                SessionEntry::Group(g) => {
                                                    app.target_group_path = Some(g.path.clone());
                                                    app.status_message = format!(
                                                         "Confirm DELETE group '{}' and all sessions? (y/n)",
                                                         g.name
                                                     );
                                                    app.input_mode = InputMode::DeleteConfirm;
                                                }
                                            }
                                        }
                                    }
                                } else if app.tab_index == 0
                                    && app.dash_focus == DashFocus::Sessions
                                {
                                    if let Some((sid, title)) = app
                                        .dash_selected_session()
                                        .map(|s| (s.session_id.clone(), s.title.clone()))
                                    {
                                        app.target_session_id = Some(sid);
                                        app.status_message =
                                            format!("Confirm DELETE session '{}'? (y/n)", title);
                                        app.input_mode = InputMode::DeleteConfirm;
                                    }
                                }
                            }
                            (_, KeyCode::Char('f')) => {
                                if app.tab_index == 1 {
                                    if let Some(i) = app.session_state.selected() {
                                        if let Some(SessionEntry::Session(s)) =
                                            app.session_entries.get(i)
                                        {
                                            app.target_session_id = Some(s.session_id.clone());
                                            app.rename_buffer = format!("{}-fork", s.title);
                                            app.input_mode = InputMode::ForkSession;
                                        }
                                    }
                                }
                            }
                            (_, KeyCode::Char('l')) => {
                                if app.tab_index == 7 {
                                    // LSPs tab - view logs
                                    if let Some((session_id, lsp_name, _status)) =
                                        app.get_selected_lsp()
                                    {
                                        app.read_lsp_logs(&session_id, &lsp_name);
                                        app.input_mode = InputMode::McpLogs; // Reuse the existing log viewer
                                        app.status_message = format!(
                                            "Viewing logs for '{}' (press Esc to close)",
                                            lsp_name
                                        );
                                    } else {
                                        app.status_message = "No LSP selected".to_string();
                                    }
                                }
                            }
                            (_, KeyCode::Char('i')) if app.tab_index == 7 => {
                                // LSPs tab - open installer modal
                                app.lsp_installer.is_open = true;
                                app.lsp_installer.selected_index = 0;
                                app.input_mode = InputMode::LspInstaller;
                                app.status_message = "LSP Installer - Select an LSP to install".to_string();
                            }
                            // Ktop-specific keybindings (use Alt to avoid conflicts)
                            (KeyModifiers::ALT, KeyCode::Char('p')) if app.tab_index == 6 => {
                                // Ktop tab - pause/resume
                                if let Some(ref mut ktop) = app.ktop_state {
                                    ktop.toggle_pause();
                                    app.status_message = if ktop.paused {
                                        "Krustop paused".to_string()
                                    } else {
                                        "Krustop resumed".to_string()
                                    };
                                }
                            }
                            (KeyModifiers::ALT, KeyCode::Char('+') | KeyCode::Char('=')) => {
                                if app.tab_index == 6 {
                                    // Ktop tab - increase refresh rate
                                    if let Some(ref mut ktop) = app.ktop_state {
                                        ktop.set_refresh_interval(ktop.refresh_interval_secs + 1);
                                        app.status_message = format!(
                                            "Refresh interval: {}s",
                                            ktop.refresh_interval_secs
                                        );
                                    }
                                }
                            }
                            (KeyModifiers::ALT, KeyCode::Char('-') | KeyCode::Char('_')) => {
                                if app.tab_index == 6 {
                                    // Ktop tab - decrease refresh rate
                                    if let Some(ref mut ktop) = app.ktop_state {
                                        ktop.set_refresh_interval(ktop.refresh_interval_secs.saturating_sub(1).max(1));
                                        app.status_message = format!(
                                            "Refresh interval: {}s",
                                            ktop.refresh_interval_secs
                                        );
                                    }
                                }
                            }
                            (KeyModifiers::ALT, KeyCode::Tab) => {
                                if app.tab_index == 6 {
                                    // Ktop tab - cycle section focus
                                    if let Some(ref mut ktop) = app.ktop_state {
                                        use crate::tabs::ktop::KtopFocus;
                                        ktop.focus = match ktop.focus {
                                            KtopFocus::Cpu => KtopFocus::Memory,
                                            KtopFocus::Memory => KtopFocus::Processes,
                                            KtopFocus::Processes => KtopFocus::Network,
                                            KtopFocus::Network => KtopFocus::Disk,
                                            KtopFocus::Disk => KtopFocus::Maestro,
                                            KtopFocus::Maestro => KtopFocus::Cpu,
                                        };
                                    }
                                }
                            }
                            (KeyModifiers::ALT | KeyModifiers::SHIFT, KeyCode::BackTab) => {
                                if app.tab_index == 6 {
                                    // Ktop tab - reverse cycle section focus
                                    if let Some(ref mut ktop) = app.ktop_state {
                                        use crate::tabs::ktop::KtopFocus;
                                        ktop.focus = match ktop.focus {
                                            KtopFocus::Cpu => KtopFocus::Maestro,
                                            KtopFocus::Memory => KtopFocus::Cpu,
                                            KtopFocus::Processes => KtopFocus::Memory,
                                            KtopFocus::Network => KtopFocus::Processes,
                                            KtopFocus::Disk => KtopFocus::Network,
                                            KtopFocus::Maestro => KtopFocus::Disk,
                                        };
                                    }
                                }
                            }
                            (_, KeyCode::Char('d')) if app.tab_index == 7 || app.tab_index == 6 => {
                                if app.tab_index == 7 {
                                    // LSPs tab - open diagnostic detail view
                                    app.diagnostic_view.is_open = true;
                                    app.diagnostic_view.selected_index = 0;
                                    app.input_mode = InputMode::DiagnosticView;
                                    app.status_message = "Diagnostic Details - Press 'S' to send to agent".to_string();
                                }
                            }
                            (_, KeyCode::Char('S')) => {
                                if app.tab_index == 6 && app.diagnostic_view.is_open {
                                    // Send diagnostics to agent
                                    let project_path = app.sessions.first()
                                        .map(|s| s.project_path.clone())
                                        .unwrap_or_else(|| ".".to_string());
                                    let prompt = crate::tabs::lsps::generate_agent_prompt(
                                        &app.lsp_diagnostics_cache,
                                        &project_path
                                    );
                                    // Copy to clipboard
                                    let _ = cli_clipboard::set_contents(prompt.clone());
                                    app.status_message = "Diagnostics copied to clipboard! Paste in your agent session.".to_string();
                                }
                            }
                            (_, KeyCode::Char('u') | KeyCode::Char('U')) => {
                                if app.tab_index == 1 {
                                    let Some(i) = app.session_state.selected() else {
                                        continue;
                                    };
                                    let Some(SessionEntry::Session(s)) =
                                        app.session_entries.get(i).cloned()
                                    else {
                                        continue;
                                    };

                                    let Some(svc) = service.as_ref() else {
                                        app.status_message =
                                            "Error: Memory service not available".to_string();
                                        continue;
                                    };

                                    app.is_spawning = true;
                                    app.status_message =
                                        format!("Resuming '{}' (agent + shell)...", s.title);
                                    let _ = terminal.draw(|frame| ui(frame, &mut app));

                                    let s_clone = s.clone();
                                    let svc_clone = svc.clone();
                                    let lsp_manager = app.lsp_manager.clone();
                                    let res = tokio::task::spawn_blocking(move || {
                                        let mut manager = leindex_core::memory::session_manager::SessionManager::new(svc_clone)?;

                                        if let Some(lsp_manager) = lsp_manager {
                                            manager = manager.with_lsp_manager(lsp_manager);
                                        }

                                        manager.restore_session(
                                            &s_clone,
                                            leindex_core::memory::session_manager::SessionRestoreMode::Resume,
                                        )
                                    }).await.ok().and_then(|r: Result<(), anyhow::Error>| r.ok());
                                    app.is_spawning = false;
                                    app.refresh_from_service(&service);

                                    match res {
                                        Some(()) => {
                                            app.status_message = format!(
                                                "Attaching to '{}'... (Ctrl+B d to detach)",
                                                s.title
                                            );
                                            let _ = terminal.draw(|frame| ui(frame, &mut app));
                                            let _ = suspend_fullscreen_app(terminal);
                                            let attach_res = TmuxMultiplexer::attach(&s.session_id);
                                            let _ = resume_fullscreen_app(terminal);
                                            let _ = terminal.clear();
                                            app.status_message = match attach_res {
                                                Ok(()) => format!("Returned from '{}'", s.title),
                                                Err(e) => format!("Attach failed: {}", e),
                                            };
                                        }
                                        None => {
                                            app.status_message = format!("Resume failed");
                                        }
                                    }
                                }
                            }
                            (KeyModifiers::ALT, KeyCode::Char('d'))
                            | (KeyModifiers::ALT, KeyCode::Char('D'))
                            | (KeyModifiers::NONE, KeyCode::Char('d'))
                                if app.tab_index != 4 && app.tab_index != 6 && app.tab_index != 7 =>
                            {
                                if app.tab_index == 1 {
                                    if let Some(i) = app.session_state.selected() {
                                        if let Some(entry) = app.session_entries.get(i) {
                                            match entry {
                                                SessionEntry::Session(s) => {
                                                    app.target_session_id =
                                                        Some(s.session_id.clone());
                                                    app.status_message = format!(
                                                         "Confirm PERMANENT DELETE session '{}'? (y/n)",
                                                         s.title
                                                     );
                                                    app.input_mode = InputMode::DeleteConfirm;
                                                }
                                                SessionEntry::Group(g) => {
                                                    app.target_group_path = Some(g.path.clone());
                                                    app.status_message = format!(
                                                         "Confirm DELETE group '{}' and all sessions? (y/n)",
                                                         g.name
                                                     );
                                                    app.input_mode = InputMode::DeleteConfirm;
                                                }
                                            }
                                        }
                                    }
                                } else if app.tab_index == 0
                                    && app.dash_focus == DashFocus::Sessions
                                {
                                    if let Some((session_id, title)) = app
                                        .dash_selected_session()
                                        .map(|s| (s.session_id.clone(), s.title.clone()))
                                    {
                                        app.target_session_id = Some(session_id);
                                        app.status_message = format!(
                                            "Confirm PERMANENT DELETE session '{}'? (y/n)",
                                            title
                                        );
                                        app.input_mode = InputMode::DeleteConfirm;
                                    }
                                } else if app.tab_index == 2 {
                                    // Project list temporary message
                                    app.status_message =
                                        "Project deletion via TUI coming soon in v2.1".to_string();
                                }
                            }
                            (KeyModifiers::ALT, KeyCode::Char('o')) => {
                                app.tab_index = if app.tab_index == 0 {
                                    6
                                } else {
                                    app.tab_index - 1
                                };
                                app.preview_focused = false;
                            }
                            (KeyModifiers::ALT, KeyCode::Char('i')) => {
                                app.tab_index = (app.tab_index + 1) % 9; // 9 tabs now (including Settings)
                                app.preview_focused = false;
                            }
                            (_, KeyCode::Down) => {
                                if app.input_mode == InputMode::LspInstaller {
                                    let count = crate::tabs::lsp_registry::get_available_lsps().len();
                                    if app.lsp_installer.selected_index < count.saturating_sub(1) {
                                        app.lsp_installer.selected_index += 1;
                                    } else {
                                        app.lsp_installer.selected_index = 0;
                                    }
                                } else if app.input_mode == InputMode::DiagnosticView {
                                    let count = app.lsp_diagnostics_cache.len();
                                    if count > 0 && app.diagnostic_view.selected_index < count.saturating_sub(1) {
                                        app.diagnostic_view.selected_index += 1;
                                    } else {
                                        app.diagnostic_view.selected_index = 0;
                                    }
                                } else if app.input_mode == InputMode::McpMenu {
                                    app.mcp_menu_option = match app.mcp_menu_option {
                                        McpOption::Start => McpOption::Stop,
                                        McpOption::Stop => McpOption::Pause,
                                        McpOption::Pause => McpOption::Logs,
                                        McpOption::Logs => McpOption::Add,
                                        McpOption::Add => McpOption::Remove,
                                        McpOption::Remove => McpOption::Start,
                                    };
                                } else if app.preview_focused {
                                    app.preview_scroll = app.preview_scroll.saturating_add(1);
                                } else if app.tab_index == 0 {
                                    // Dashboard
                                    match app.dash_focus {
                                        DashFocus::Sessions => {
                                            app.dash_select_next_session();
                                        }
                                        DashFocus::Mcp => {
                                            let i = match app.mcp_state.selected() {
                                                Some(i) => {
                                                    if i >= app.mcp_servers.len().saturating_sub(1)
                                                    {
                                                        0
                                                    } else {
                                                        i + 1
                                                    }
                                                }
                                                None => 0,
                                            };
                                            app.mcp_state.select(Some(i));
                                        }
                                        DashFocus::Tabs => {}
                                    }
                                } else if app.tab_index == 2 {
                                    // Projects
                                    if app.preview_focused {
                                        app.project_explorer_selected =
                                            (app.project_explorer_selected + 1)
                                                % app.explorer_items.len().max(1);
                                    } else {
                                        let i = match app.project_state.selected() {
                                            Some(i) => {
                                                if i >= app.projects.len().saturating_sub(1) {
                                                    0
                                                } else {
                                                    i + 1
                                                }
                                            }
                                            None => 0,
                                        };
                                        app.project_state.select(Some(i));
                                        app.project_explorer_path = None;
                                        app.project_explorer_selected = 0;
                                    }
                                } else if app.tab_index == 1 {
                                    // Sessions Tab
                                    let i = match app.session_state.selected() {
                                        Some(i) => {
                                            if i >= app.session_entries.len().saturating_sub(1) {
                                                0
                                            } else {
                                                i + 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    app.session_state.select(Some(i));
                                } else if app.tab_index == 5 {
                                    // Memory
                                    let i = match app.memory_state.selected() {
                                        Some(i) => {
                                            if i >= app.memories.len().saturating_sub(1) {
                                                0
                                            } else {
                                                i + 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    app.memory_state.select(Some(i));
                                } else if app.tab_index == 6 {
                                    // LSPs
                                    let i = match app.lsp_state.selected() {
                                        Some(i) => {
                                            if i >= app
                                                .lsp_status_cache
                                                .values()
                                                .map(|v| v.len())
                                                .sum::<usize>()
                                                .saturating_sub(1)
                                            {
                                                0
                                            } else {
                                                i + 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    app.lsp_state.select(Some(i));
                                } else if app.tab_index == 8 {
                                    // Settings
                                    app.settings_option = match app.settings_option {
                                        SettingsOption::Editor => SettingsOption::Theme,
                                        SettingsOption::Theme => SettingsOption::Transparent,
                                        SettingsOption::Transparent => SettingsOption::InstallPath,
                                        SettingsOption::InstallPath => SettingsOption::Save,
                                        SettingsOption::Save => SettingsOption::Editor,
                                    };
                                }
                                app.scroll = app.scroll.saturating_add(1);
                            }
                            (_, KeyCode::Up) => {
                                if app.input_mode == InputMode::LspInstaller {
                                    let count = crate::tabs::lsp_registry::get_available_lsps().len();
                                    if app.lsp_installer.selected_index > 0 {
                                        app.lsp_installer.selected_index -= 1;
                                    } else {
                                        app.lsp_installer.selected_index = count.saturating_sub(1);
                                    }
                                } else if app.input_mode == InputMode::DiagnosticView {
                                    let count = app.lsp_diagnostics_cache.len();
                                    if count > 0 && app.diagnostic_view.selected_index > 0 {
                                        app.diagnostic_view.selected_index -= 1;
                                    } else if count > 0 {
                                        app.diagnostic_view.selected_index = count.saturating_sub(1);
                                    }
                                } else if app.input_mode == InputMode::McpMenu {
                                    app.mcp_menu_option = match app.mcp_menu_option {
                                        McpOption::Start => McpOption::Remove,
                                        McpOption::Stop => McpOption::Start,
                                        McpOption::Pause => McpOption::Stop,
                                        McpOption::Logs => McpOption::Pause,
                                        McpOption::Add => McpOption::Logs,
                                        McpOption::Remove => McpOption::Add,
                                    };
                                } else if app.preview_focused {
                                    app.preview_scroll = app.preview_scroll.saturating_sub(1);
                                } else if app.tab_index == 0 {
                                    // Dashboard
                                    match app.dash_focus {
                                        DashFocus::Sessions => {
                                            app.dash_select_prev_session();
                                        }
                                        DashFocus::Mcp => {
                                            let i = match app.mcp_state.selected() {
                                                Some(i) => {
                                                    if i == 0 {
                                                        app.mcp_servers.len().saturating_sub(1)
                                                    } else {
                                                        i - 1
                                                    }
                                                }
                                                None => 0,
                                            };
                                            app.mcp_state.select(Some(i));
                                        }
                                        DashFocus::Tabs => {}
                                    }
                                } else if app.tab_index == 2 {
                                    // Projects Tab
                                    if app.preview_focused {
                                        app.project_explorer_selected =
                                            if app.project_explorer_selected == 0 {
                                                app.explorer_items.len().saturating_sub(1)
                                            } else {
                                                app.project_explorer_selected - 1
                                            };
                                    } else {
                                        let i = match app.project_state.selected() {
                                            Some(i) => {
                                                if i == 0 {
                                                    app.projects.len().saturating_sub(1)
                                                } else {
                                                    i - 1
                                                }
                                            }
                                            None => 0,
                                        };
                                        app.project_state.select(Some(i));
                                        app.project_explorer_path = None;
                                        app.project_explorer_selected = 0;
                                    }
                                } else if app.tab_index == 1 {
                                    // Sessions Tab
                                    let i = match app.session_state.selected() {
                                        Some(i) => {
                                            if i == 0 {
                                                app.session_entries.len().saturating_sub(1)
                                            } else {
                                                i - 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    app.session_state.select(Some(i));
                                } else if app.tab_index == 5 {
                                    // Memory
                                    let i = match app.memory_state.selected() {
                                        Some(i) => {
                                            if i == 0 {
                                                app.memories.len().saturating_sub(1)
                                            } else {
                                                i - 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    app.memory_state.select(Some(i));
                                } else if app.tab_index == 6 {
                                    // LSPs
                                    let i = match app.lsp_state.selected() {
                                        Some(i) => {
                                            let total_lsps = app
                                                .lsp_status_cache
                                                .values()
                                                .map(|v| v.len())
                                                .sum::<usize>();
                                            if i == 0 {
                                                total_lsps.saturating_sub(1)
                                            } else {
                                                i - 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    app.lsp_state.select(Some(i));
                                } else if app.tab_index == 8 {
                                    // Settings
                                    app.settings_option = match app.settings_option {
                                        SettingsOption::Editor => SettingsOption::Save,
                                        SettingsOption::Save => SettingsOption::InstallPath,
                                        SettingsOption::InstallPath => SettingsOption::Transparent,
                                        SettingsOption::Transparent => SettingsOption::Theme,
                                        SettingsOption::Theme => SettingsOption::Editor,
                                    };
                                }
                                app.scroll = app.scroll.saturating_sub(1);
                            }
                            (_, KeyCode::Char('G')) => {
                                if app.tab_index == 1 {
                                    app.input_mode = InputMode::NewGroupTitle;
                                    app.rename_buffer.clear();
                                    app.new_group_category.clear();
                                }
                            }
                            (KeyModifiers::ALT, KeyCode::Char('c')) => {
                                if app.tab_index == 1 {
                                    app.input_mode = InputMode::NewGroupTitle;
                                    app.rename_buffer.clear();
                                    app.new_group_category.clear();
                                }
                            }
                            (_, KeyCode::Char('m')) => {
                                if app.tab_index == 1 {
                                    if let Some(i) = app.session_state.selected() {
                                        if let Some(SessionEntry::Session(s)) =
                                            app.session_entries.get(i)
                                        {
                                            app.target_session_id = Some(s.session_id.clone());
                                            app.input_mode = InputMode::MoveToGroup;
                                            app.rename_buffer.clear();
                                        }
                                    }
                                }
                            }
                            (_, KeyCode::Char('a')) => {
                                if app.tab_index == 3 {
                                    // Analysis tab
                                    app.input_mode = InputMode::AnalysisPrompt;
                                }
                            }
                            (_, KeyCode::Right) => {
                                if app.tab_index == 2 && app.preview_focused {
                                    if let (Some(_), Some(path)) =
                                        (app.project_state.selected(), &app.project_explorer_path)
                                    {
                                        if let Some(item_name) =
                                            app.explorer_items.get(app.project_explorer_selected)
                                        {
                                            let mut new_path =
                                                std::path::PathBuf::from(path.replace(
                                                    "~",
                                                    &std::env::var("HOME").unwrap_or_default(),
                                                ));
                                            new_path.push(item_name);
                                            if new_path.is_dir() {
                                                app.project_explorer_path =
                                                    Some(new_path.to_string_lossy().to_string());
                                                app.project_explorer_selected = 0;
                                            } else {
                                                let _ = terminal.clear();
                                                let editor = &app.config.editor;
                                                let _ = std::process::Command::new(editor)
                                                    .arg(new_path)
                                                    .status();
                                                let _ = terminal.clear();
                                            }
                                        }
                                    }
                                } else if app.tab_index == 1 {
                                    if let Some(i) = app.session_state.selected() {
                                        if let Some(SessionEntry::Group(g)) =
                                            app.session_entries.get(i)
                                        {
                                            if !g.is_expanded {
                                                if let Some(group) = app
                                                    .groups
                                                    .iter_mut()
                                                    .find(|group| group.path == g.path)
                                                {
                                                    group.is_expanded = true;
                                                    if let Some(svc) = service.as_ref() {
                                                        let _ = svc.update_group_expansion(
                                                            &group.path,
                                                            true,
                                                        );
                                                    }
                                                    app.refresh_session_entries();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            (_, KeyCode::Left) => {
                                if app.tab_index == 2 && app.preview_focused {
                                    if let Some(current) = &app.project_explorer_path {
                                        let path = std::path::PathBuf::from(current);
                                        if let Some(parent) = path.parent() {
                                            app.project_explorer_path =
                                                Some(parent.to_string_lossy().to_string());
                                            app.project_explorer_selected = 0;
                                        }
                                    }
                                } else if app.tab_index == 1 {
                                    if let Some(i) = app.session_state.selected() {
                                        if let Some(SessionEntry::Group(g)) =
                                            app.session_entries.get(i)
                                        {
                                            if g.is_expanded {
                                                if let Some(group) = app
                                                    .groups
                                                    .iter_mut()
                                                    .find(|group| group.path == g.path)
                                                {
                                                    group.is_expanded = false;
                                                    if let Some(svc) = service.as_ref() {
                                                        let _ = svc.update_group_expansion(
                                                            &group.path,
                                                            false,
                                                        );
                                                    }
                                                    app.refresh_session_entries();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            (_, KeyCode::Backspace)
                                if app.tab_index == 2 && app.preview_focused =>
                            {
                                if let Some(current) = &app.project_explorer_path {
                                    let path =
                                        std::path::PathBuf::from(current.replace(
                                            "~",
                                            &std::env::var("HOME").unwrap_or_default(),
                                        ));
                                    if let Some(parent) = path.parent() {
                                        app.project_explorer_path =
                                            Some(parent.to_string_lossy().to_string());
                                        app.project_explorer_selected = 0;
                                    }
                                }
                            }
                            (_, KeyCode::Char('/') | KeyCode::Char('h') | KeyCode::Char('?')) => {
                                app.show_help = true;
                                app.help_scroll = 0;
                            }
                            (_, KeyCode::Char('e')) if app.tab_index == 2 => {
                                app.preview_focused = !app.preview_focused;
                            }
                            (_, KeyCode::Enter) if app.tab_index != 4 => {
                                if app.tab_index == 0 {
                                    // Dashboard
                                    match app.dash_focus {
                                        DashFocus::Sessions => {
                                            if let Some(s) = app.dash_selected_session().cloned() {
                                                app.status_message = format!(
                                                    "Attaching to '{}'... (Ctrl+B d to detach)",
                                                    s.title
                                                );
                                                let _ = terminal.draw(|frame| ui(frame, &mut app));
                                                let _ = suspend_fullscreen_app(terminal);
                                                let res = TmuxMultiplexer::attach(&s.session_id);
                                                let _ = resume_fullscreen_app(terminal);
                                                let _ = terminal.clear();
                                                app.status_message = match res {
                                                    Ok(()) => {
                                                        format!("Returned from '{}'", s.title)
                                                    }
                                                    Err(e) => format!("Attach failed: {}", e),
                                                };
                                            }
                                        }
                                        DashFocus::Mcp => {
                                            if let Some(i) = app.mcp_state.selected() {
                                                if let Some(mcp) = app.mcp_servers.get(i) {
                                                    app.target_mcp_name = Some(mcp.name.clone());
                                                    app.input_mode = InputMode::McpMenu;
                                                    app.mcp_menu_option = McpOption::Start;
                                                }
                                            }
                                        }
                                        DashFocus::Tabs => {}
                                    }
                                } else if app.tab_index == 8 {
                                    // Settings
                                    match app.settings_option {
                                        SettingsOption::Editor => {
                                            app.open_settings_menu(SettingsMenuKind::Editor);
                                        }
                                        SettingsOption::InstallPath => {
                                            app.rename_buffer = app.config.install_path.clone();
                                            app.input_mode = InputMode::SettingsInstallPath;
                                        }
                                        SettingsOption::Theme => {
                                            app.open_settings_menu(SettingsMenuKind::Theme);
                                        }
                                        SettingsOption::Transparent => {
                                            app.config.transparent = !app.config.transparent;
                                            // Auto-save config when transparency is toggled
                                            if let Err(e) = app.config.save() {
                                                app.status_message =
                                                    format!("Failed to save config: {}", e);
                                            } else {
                                                app.status_message = if app.config.transparent {
                                                    "Transparency enabled".to_string()
                                                } else {
                                                    "Transparency disabled".to_string()
                                                };
                                            }
                                        }
                                        SettingsOption::Save => {
                                            let _ = app.config.save();
                                            app.status_message = "Configuration saved to ~/.config/maestro/config.toml".to_string();
                                        }
                                    }
                                } else if app.tab_index == 2 {
                                    // Projects Tab
                                    if let Some(i) = app.project_state.selected() {
                                        let project = &app.projects[i].clone();
                                        app.status_message =
                                            format!("Launching Zide for {}...", project.name);
                                        let _ = terminal.draw(|frame| ui(frame, &mut app));

                                        let _ = suspend_fullscreen_app(terminal);
                                        let res = leindex_core::multiplexer::zellij::ZellijMultiplexer::spawn_zide(&project.path, &project.name);
                                        let _ = resume_fullscreen_app(terminal);

                                        match res {
                                            Ok(_) => {
                                                let _ = terminal.clear(); // Ensure screen is clear after Zellij exit
                                                let _ = terminal.draw(|frame| ui(frame, &mut app));
                                                app.status_message = format!(
                                                    "Returned from Zide for {}.",
                                                    project.name
                                                );
                                            }
                                            Err(e) => {
                                                app.status_message = format!("Error: {}", e);
                                            }
                                        }
                                    }
                                } else if app.tab_index == 1 {
                                    // Sessions Tab
                                    if let Some(i) = app.session_state.selected() {
                                        if let Some(entry) = app.session_entries.get(i).cloned() {
                                            match entry {
                                                SessionEntry::Group(g) => {
                                                    if let Some(group) = app
                                                        .groups
                                                        .iter_mut()
                                                        .find(|group| group.path == g.path)
                                                    {
                                                        group.is_expanded = !group.is_expanded;
                                                        if let Some(svc) = service.as_ref() {
                                                            let _ = svc.update_group_expansion(
                                                                &group.path,
                                                                group.is_expanded,
                                                            );
                                                        }
                                                        app.refresh_session_entries();
                                                    }
                                                }
                                                SessionEntry::Session(s) => {
                                                    app.status_message = format!(
                                                        "Attaching to '{}'... (Ctrl+B d to detach)",
                                                        s.title
                                                    );
                                                    let _ =
                                                        terminal.draw(|frame| ui(frame, &mut app));
                                                    let _ = suspend_fullscreen_app(terminal);
                                                    let res =
                                                        TmuxMultiplexer::attach(&s.session_id);
                                                    let _ = resume_fullscreen_app(terminal);
                                                    let _ = terminal.clear(); // Restore terminal state
                                                    match res {
                                                        Ok(()) => {
                                                            app.status_message = format!(
                                                                "Returned from '{}'",
                                                                s.title
                                                            );
                                                        }
                                                        Err(e) => {
                                                            // If the session is dead, do the useful thing:
                                                            // recreate the shell and attempt to resume the agent (best-effort).
                                                            if s.status
	                                                                == leindex_core::memory::models::SessionStatus::Terminated
	                                                            {
	                                                                if let Some(svc) = service.as_ref()
	                                                                {
	                                                                    app.is_spawning = true;
	                                                                    app.status_message = format!(
	                                                                        "Session terminated; resuming '{}'...",
	                                                                        s.title
	                                                                    );
	                                                                    let _ = terminal.draw(|frame| {
	                                                                        ui(frame, &mut app)
	                                                                    });

	                                                                    let s_clone = s.clone();
	                                                                    let svc_clone = svc.clone();
	                                                                    let lsp_manager = app.lsp_manager.clone();
	                                                                    let _ = tokio::task::spawn_blocking(move || {
	                                                                        let mut manager = leindex_core::memory::session_manager::SessionManager::new(svc_clone)?;

	                                                                        if let Some(lsp_manager) = lsp_manager {
	                                                                            manager = manager.with_lsp_manager(lsp_manager);
	                                                                        }

	                                                                        manager.restore_session(
	                                                                            &s_clone,
	                                                                            leindex_core::memory::session_manager::SessionRestoreMode::Resume,
	                                                                        )
	                                                                    }).await.ok().and_then(|r: Result<(), anyhow::Error>| r.ok());
	                                                                    app.is_spawning = false;
	                                                                    app.refresh_from_service(&service);

	                                                                    app.status_message = format!(
	                                                                        "Attaching to '{}'... (Ctrl+B d to detach)",
	                                                                        s.title
	                                                                    );
	                                                                    let _ = terminal.draw(|frame| {
	                                                                        ui(frame, &mut app)
	                                                                    });
	                                                                    let _ = suspend_fullscreen_app(terminal);
	                                                                    let attach_res =
	                                                                        TmuxMultiplexer::attach(&s.session_id);
	                                                                    let _ =
	                                                                        resume_fullscreen_app(terminal);
	                                                                    let _ = terminal.clear();
	                                                                    app.status_message =
	                                                                        match attach_res {
	                                                                            Ok(()) => format!(
	                                                                                "Returned from '{}'",
	                                                                                s.title
	                                                                            ),
	                                                                            Err(e) => format!(
	                                                                                "Attach failed: {}",
	                                                                                e
	                                                                            ),
	                                                                        };
	                                                                } else {
	                                                                    app.status_message =
	                                                                        format!("Attach failed: {}", e);
	                                                                }
	                                                            } else {
	                                                                app.status_message =
	                                                                    format!("Attach failed: {}", e);
	                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            (_, KeyCode::Char('s')) if app.tab_index != 4 => {
                                if app.tab_index == 0 {
                                    // Dashboard (MCP)
                                    if let Some(i) = app.mcp_state.selected() {
                                        if let Some(mcp) = app.mcp_servers.get(i) {
                                            app.target_mcp_name = Some(mcp.name.clone());
                                            app.input_mode = InputMode::McpMenu;
                                            app.mcp_menu_option = McpOption::Start;
                                        }
                                    }
                                } else if app.tab_index == 6 {
                                    // LSPs tab
                                    // Toggle LSP start/stop
                                    if let Some((session_id, lsp_name, status)) =
                                        app.get_selected_lsp()
                                    {
                                        app.toggle_lsp(&session_id, &lsp_name, status);
                                    } else {
                                        let scheduled = app.queue_lsp_autostart_for_sessions();
                                        app.status_message = if scheduled {
                                            "Queued LSP auto-detect for active sessions".to_string()
                                        } else {
                                            "No LSPs detected yet. Press 'r' to rescan.".to_string()
                                        };
                                    }
                                } else {
                                    app.input_mode = InputMode::SessionSwitcher;
                                    app.switcher_state.select(Some(0));
                                }
                            }
                            (_, KeyCode::Char('x')) => {
                                if app.tab_index == 0 {
                                    // Remove MCP
                                    if let Some(i) = app.mcp_state.selected() {
                                        let name = app.mcp_servers[i].name.clone();
                                        if let Some(svc) = service.as_ref() {
                                            let _ = svc.delete_mcp_server(&name);
                                            if let Ok(mcp_list) = svc.list_mcp_servers() {
                                                app.mcp_servers = mcp_list;
                                            }
                                        }
                                    }
                                }
                            }
                            (_, KeyCode::Char('R')) => {
                                // Sessions tab: restart the selected session (shell/tool fresh).
                                if app.tab_index == 1 {
                                    let Some(i) = app.session_state.selected() else {
                                        continue;
                                    };
                                    let Some(SessionEntry::Session(s)) =
                                        app.session_entries.get(i).cloned()
                                    else {
                                        continue;
                                    };

                                    let Some(svc) = service.as_ref() else {
                                        app.status_message =
                                            "Error: Memory service not available".to_string();
                                        continue;
                                    };

                                    app.is_spawning = true;
                                    app.status_message =
                                        format!("Restarting '{}' (shell + fresh tool)...", s.title);
                                    let _ = terminal.draw(|frame| ui(frame, &mut app));

                                    let s_clone = s.clone();
                                    let svc_clone = svc.clone();
                                    let lsp_manager = app.lsp_manager.clone();
                                    let res = tokio::task::spawn_blocking(move || {
                                        let mut manager = leindex_core::memory::session_manager::SessionManager::new(svc_clone)?;

                                        if let Some(lsp_manager) = lsp_manager {
                                            manager = manager.with_lsp_manager(lsp_manager);
                                        }

                                        manager.restore_session(
                                            &s_clone,
                                            leindex_core::memory::session_manager::SessionRestoreMode::Restart,
                                        )
                                    }).await.ok().and_then(|r: Result<(), anyhow::Error>| r.ok());
                                    app.is_spawning = false;
                                    app.refresh_from_service(&service);

                                    match res {
                                        Some(()) => {
                                            app.status_message = format!(
                                                "Attaching to '{}'... (Ctrl+B d to detach)",
                                                s.title
                                            );
                                            let _ = terminal.draw(|frame| ui(frame, &mut app));
                                            let _ = suspend_fullscreen_app(terminal);
                                            let attach_res = TmuxMultiplexer::attach(&s.session_id);
                                            let _ = resume_fullscreen_app(terminal);
                                            let _ = terminal.clear();
                                            app.status_message = match attach_res {
                                                Ok(()) => format!("Returned from '{}'", s.title),
                                                Err(e) => format!("Attach failed: {}", e),
                                            };
                                        }
                                        None => {
                                            app.status_message = format!("Restart failed");
                                        }
                                    }
                                } else if app.tab_index == 6 {
                                    // LSPs tab
                                    // Restart LSP
                                    if let Some((session_id, lsp_name, _status)) =
                                        app.get_selected_lsp()
                                    {
                                        app.restart_lsp(&session_id, &lsp_name);
                                    } else {
                                        app.status_message = "No LSP selected".to_string();
                                    }
                                } else {
                                    // Other tabs: manual full refresh
                                    if let Some(svc) = service.as_ref() {
                                        let _ = svc.sync_mcp_servers_from_system();
                                        let _ = svc.sync_memories_from_system();
                                        if let Ok(projects) = svc.list_projects() {
                                            app.projects = projects
                                                .iter()
                                                .map(|p| ProjectInfo {
                                                    name: p.project_name.clone(),
                                                    path: p.project_path.clone(),
                                                    _track_count: 0,
                                                })
                                                .collect();
                                        }
                                        app.refresh_from_service(&service);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn ui(frame: &mut Frame, app: &mut App) {
    let theme = app.theme();
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.bg).fg(theme.fg)),
        frame.area(),
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    // Header with tabs
    let is_focused = app.tab_index == 0 && app.dash_focus == DashFocus::Tabs;
    let tabs = Tabs::new(vec![
        "Dashboard",
        "Sessions",
        "Projects",
        "Analysis",
        "Conductor",
        "Memory",
        "Ktop",
        "LSPs",
        "Settings",
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(if is_focused {
                BorderType::Double
            } else {
                BorderType::Rounded
            })
            .border_style(if is_focused {
                Style::default().fg(theme.warning).bold()
            } else {
                Style::default().fg(theme.muted)
            })
            .title(" Maestro Cockpit v2.0 "),
    )
    .select(app.tab_index)
    .highlight_style(Style::default().fg(theme.accent).bold());

    frame.render_widget(tabs, chunks[0]);

    match app.tab_index {
        0 => render_dashboard(frame, chunks[1], app),
        1 => render_sessions(frame, chunks[1], app),
        2 => render_projects(frame, chunks[1], app),
        3 => render_analysis(frame, chunks[1], app),
        4 => {
            let theme = app.theme();
            // Sync active sessions from Sessions tab into conductor tracks
            app.conductor.sync_sessions_as_tracks(&app.sessions);
            crate::conductor::render_conductor(frame, chunks[1], &mut app.conductor, &theme);
        }
        5 => render_memory(frame, chunks[1], app),
        6 => crate::tabs::ktop::render_ktop(frame, chunks[1], app),
        7 => render_lsps(frame, chunks[1], app),
        8 => render_settings(frame, app),
        _ => {}
    }
    // Footer
    let footer = Paragraph::new(vec![Line::from(vec![
        Span::styled(" Tab ", Style::default().bg(Color::Cyan).fg(Color::Black)),
        Span::raw(" Switch  "),
        Span::styled(
            " ↑↓ Arrows ",
            Style::default().bg(Color::Cyan).fg(Color::Black),
        ),
        Span::raw(" Scroll  "),
        Span::styled(
            " Alt+1-8 ",
            Style::default().bg(Color::Cyan).fg(Color::Black),
        ),
        Span::raw(" Tabs  "),
        Span::styled(" n ", Style::default().bg(Color::Green).fg(Color::Black)),
        Span::raw(" New  "),
        Span::styled(" s ", Style::default().bg(Color::Magenta).fg(Color::Black)),
        Span::raw(" Switch "),
        Span::styled(" / ", Style::default().bg(Color::Yellow).fg(Color::Black)),
        Span::raw(" Help "),
        if std::env::var("ZELLIJ").is_ok() {
            Span::styled(
                " [Zellij Active: Ctrl+G for menu] ",
                Style::default().fg(Color::Yellow).bold(),
            )
        } else {
            Span::raw("")
        },
        Span::styled(" q ", Style::default().bg(Color::Red).fg(Color::White)),
        Span::raw(" Quit"),
    ])])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );
    frame.render_widget(footer, chunks[2]);

    // Render Modals
    if app.show_help {
        modals::render_help_modal(frame, app);
    }

    // Only show these modals if they overlay the main tabs appropriately
    if app.input_mode == InputMode::SessionSwitcher {
        modals::render_switcher_modal(frame, app);
    } else if app.input_mode == InputMode::SessionHub {
        modals::render_session_hub_modal(frame, app);
    } else if app.input_mode == InputMode::McpMenu {
        modals::render_mcp_menu(frame, app);
    } else if app.input_mode == InputMode::McpLogs {
        modals::render_mcp_logs_modal(frame, app);
    } else if app.input_mode == InputMode::SettingsMenu {
        modals::render_settings_menu_modal(frame, app);
    } else if matches!(
        app.input_mode,
        InputMode::NewProjectName | InputMode::NewProjectPath | InputMode::NewProjectTool
    ) {
        modals::render_new_project_modal(frame, app);
    } else if matches!(
        app.input_mode,
        InputMode::NewTrackTitle | InputMode::NewTrackType
    ) {
        modals::render_new_track_modal(frame, app);
    } else if matches!(
        app.input_mode,
        InputMode::NewGroupTitle
            | InputMode::NewGroupCategory
            | InputMode::RenameGroup
            | InputMode::RenameGroupCategory
    ) {
        modals::render_group_modal(frame, app);
    } else if matches!(
        app.input_mode,
        InputMode::ForkSession
            | InputMode::KillConfirm
            | InputMode::DeleteConfirm
            | InputMode::MoveToGroup
    ) {
        modals::render_action_modal(frame, app);
    } else if matches!(
        app.input_mode,
        InputMode::NewSessionTitle | InputMode::NewSessionPath | InputMode::NewSessionTool
    ) {
        modals::render_input_modal(frame, app);
    }

    if app.is_spawning {
        modals::render_spawning_overlay(frame, app);
    }
}
