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
    widgets::{Block, Borders, Paragraph, Tabs, List, ListItem, BorderType, Clear, Wrap},
};
use std::{collections::{HashMap, HashSet}, io, sync::Arc, time::Instant, collections::hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

use leindex_analyzers::memory::MemoryService;
use leindex_analyzers::memory::models::McpStatus;
use leindex_analyzers::memory::McpPool;
use leindex_analyzers::memory::LspStatus;
use leindex_analyzers::memory::lsp_manager::LspType;
use leindex_analyzers::multiplexer::TmuxMultiplexer;
use leindex_analyzers::config::Config;
use leindex_analyzers::memory::TursoStorageBackend;

use super::theme::{theme_from_name, Theme, THEMES};

pub async fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

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

    // Create TursoStorageBackend for LSP operations (before entering async loop)
    let storage_backend = match TursoStorageBackend::new(None, None).await {
        Ok(backend) => Some(Arc::new(backend)),
        Err(e) => {
            eprintln!("Warning: Failed to create storage backend for LSP operations: {}", e);
            None
        }
    };

    // Run app
    let result = run_app(&mut terminal, service, mcp_pool, storage_backend).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

struct App {
    tab_index: usize,
    should_quit: bool,
    show_help: bool,
    input_mode: InputMode,
    projects: Vec<ProjectInfo>,
    project_state: ratatui::widgets::ListState,
    memories: Vec<MemoryInfo>,
    memory_state: ratatui::widgets::ListState,
    memory_query: String,
    sessions: Vec<leindex_analyzers::memory::models::Session>,
    session_entries: Vec<SessionEntry>,
    session_state: ratatui::widgets::ListState,
    groups: Vec<leindex_analyzers::memory::models::SessionGroup>,
    mcp_servers: Vec<leindex_analyzers::memory::models::McpServer>,
    stats: Stats,
    scroll: usize,
    // Session switcher state
    switcher_state: ratatui::widgets::ListState,
    // Input fields for new session
    new_session_title: String,
    new_session_path: String,
    new_session_tool: String,
    rename_buffer: String,
    target_session_id: Option<String>,
    target_group_path: Option<String>,
    // Status & Feedback
    is_spawning: bool,
    status_message: String,
    session_preview_content: String,
    // Analysis Hub state
    analysis_input: String,
    analysis_history: Vec<String>,
    frame_count: u64,
    // Help modal state
    help_scroll: u16,
    // Throttle expensive preview capture
    last_preview_refresh: Instant,
    // Phase 11 additions
    mcp_state: ratatui::widgets::ListState,
    preview_focused: bool,
    preview_scroll: u16,
    hub_search_buffer: String,
    hub_focus: HubFocus,
    // Dashboard MCP menu state
    mcp_menu_option: McpOption,
    target_mcp_name: Option<String>,
    mcp_pool: Option<Arc<McpPool>>,
    mcp_log_lines: Vec<String>,
    mcp_log_scroll: u16,
    // Projects tab state
    project_view_open: bool,
    // Phase 15 state
    new_project_name: String,
    new_project_path: String,
    new_project_tool: String,
    new_track_title: String,
    new_track_is_master: bool,
    new_group_category: String,
    // Project Explorer state
    project_explorer_path: Option<String>,
    project_explorer_selected: usize,
    explorer_items: Vec<String>,
    config: Config,
    settings_option: SettingsOption,
    settings_menu_kind: Option<SettingsMenuKind>,
    settings_menu_state: ratatui::widgets::ListState,
    settings_menu_items: Vec<(String, String)>,
    dash_session_state: ratatui::widgets::ListState,
    dash_session_entries: Vec<DashSessionEntry>,
    dash_focus: DashFocus,
    // Phase 6: LSP Integration
    // Cache of (session_id -> Vec<(lsp_name, status)>)
    lsp_status_cache: HashMap<String, Vec<(String, LspStatus)>>,
    last_lsp_refresh: Instant,
    lsp_state: ratatui::widgets::ListState,
    // LSP log viewing
    lsp_log_content: String,
    lsp_log_scroll: u16,
    lsp_log_source: Option<(String, String)>, // (session_id, lsp_name)
    // LSP installation guidance - tracks which LSPs are available on the system
    lsp_availability: HashMap<String, bool>, // lsp_name -> is_available
    // Storage backend for LSP operations (sync access)
    storage_backend: Option<Arc<TursoStorageBackend>>,
    // Flag to trigger async LSP refresh
    pending_lsp_refresh: bool,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum InputMode {
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

#[derive(PartialEq, Eq, Clone, Copy, Default)]
enum HubFocus {
    #[default]
    Rename,
    Group,
    Search,
}

#[derive(PartialEq, Eq, Clone, Copy, Default)]
enum McpOption {
    #[default]
    StartStop,
    Pause,
    Logs,
    Add,
    Remove,
    Install,
}

#[derive(PartialEq, Eq, Clone, Copy, Default)]
enum SettingsOption {
    #[default]
    Editor,
    InstallPath,
    Theme,
    Save,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum SettingsMenuKind {
    Editor,
    Theme,
}

#[derive(PartialEq, Eq, Clone, Copy, Default)]
enum DashFocus {
    #[default]
    Sessions,
    Mcp,
    Tabs,
}

#[derive(Clone)]
enum SessionEntry {
    Group(leindex_analyzers::memory::models::SessionGroup),
    Session(leindex_analyzers::memory::models::Session),
}

#[derive(Clone)]
enum DashSessionEntry {
    GroupHeader { group_path: String },
    Session(leindex_analyzers::memory::models::Session),
}

#[derive(Clone)]
struct ProjectInfo {
    name: String,
    path: String,
    _track_count: usize,
}

#[derive(Clone)]
struct MemoryInfo {
    _id: i64,
    content: String,
    category: String,
}

#[derive(Clone, Default)]
struct Stats {
    project_count: usize,
    memory_count: usize,
    track_count: usize,
}

impl App {
	    fn new(_service: Option<&MemoryService>, mcp_pool: Option<Arc<McpPool>>, storage_backend: Option<Arc<TursoStorageBackend>>) -> Self {
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
            session_preview_content: String::new(),
            analysis_input: String::new(),
            analysis_history: Vec::new(),
            frame_count: 0,
            help_scroll: 0,
            last_preview_refresh: Instant::now(),
            mcp_state: ratatui::widgets::ListState::default(),
            preview_focused: false,
            preview_scroll: 0,
            hub_search_buffer: String::new(),
            hub_focus: HubFocus::Rename,
            mcp_menu_option: McpOption::StartStop,
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
	            storage_backend,
	            pending_lsp_refresh: false,
	        };
	        // Check LSP availability on startup
	        app.check_lsp_availability();
	        app.mcp_state.select(Some(0));
	        app.dash_session_state.select(Some(0));
	        app.memory_state.select(Some(0));
	        app.lsp_state.select(Some(0));
	        app
	    }

	    fn theme(&self) -> Theme {
	        theme_from_name(&self.config.theme)
	    }

	    fn open_settings_menu(&mut self, kind: SettingsMenuKind) {
	        self.settings_menu_kind = Some(kind);
	        self.settings_menu_items = match kind {
	            SettingsMenuKind::Editor => vec![
	                ("hx".to_string(), "Helix (hx)".to_string()),
	                ("nvim".to_string(), "Neovim (nvim)".to_string()),
	                ("vim".to_string(), "Vim (vim)".to_string()),
	                ("emacs".to_string(), "Emacs (emacs)".to_string()),
	                ("nano".to_string(), "Nano (nano)".to_string()),
	                ("micro".to_string(), "Micro (micro)".to_string()),
	                ("code".to_string(), "VS Code (code)".to_string()),
	                ("zed".to_string(), "Zed (zed)".to_string()),
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

    fn dash_selected_session(&self) -> Option<&leindex_analyzers::memory::models::Session> {
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
            idx = if idx >= len.saturating_sub(1) { 0 } else { idx + 1 };
            if matches!(self.dash_session_entries.get(idx), Some(DashSessionEntry::Session(_))) {
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
            idx = if idx == 0 { len.saturating_sub(1) } else { idx - 1 };
            if matches!(self.dash_session_entries.get(idx), Some(DashSessionEntry::Session(_))) {
                self.dash_session_state.select(Some(idx));
                return;
            }
        }
    }

    // LSP installation guidance helpers
    fn check_lsp_availability(&mut self) {
        let lsps = vec![
            "rust-analyzer",
            "ruff-lsp",
            "typescript-language-server",
        ];

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
            if let Ok(output) = std::process::Command::new("where")
                .arg(name)
                .output()
            {
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
            if let Ok(output) = std::process::Command::new("which")
                .arg(name)
                .output()
            {
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

    fn get_lsp_install_command(lsp_name: &str) -> Vec<&'static str> {
        match lsp_name {
            "rust-analyzer" => vec![
                "# Via rustup (recommended):",
                "rustup component add rust-analyzer",
                "",
                "# Or pre-built binary:",
                "curl -L https://github.com/rust-lang/rust-analyzer/releases/latest/download/rust-analyzer-x86_64-unknown-linux-gnu -o ~/.local/bin/rust-analyzer",
                "chmod +x ~/.local/bin/rust-analyzer",
            ],
            "ruff-lsp" => vec![
                "# Via pip:",
                "pip install ruff-lsp",
                "",
                "# Or via pipx (recommended for isolation):",
                "pipx install ruff-lsp",
            ],
            "typescript-language-server" => vec![
                "# Via npm:",
                "npm install -g typescript-language-server",
                "",
                "# Or via yarn:",
                "yarn global add typescript-language-server",
            ],
            _ => vec![
                "# See LSP documentation for installation instructions",
            ],
        }
    }

    fn refresh_from_service(&mut self, service: &Option<MemoryService>) {
        if let Some(svc) = service {
            if let Ok(projects) = svc.list_projects() {
                self.projects = projects.iter().map(|p| ProjectInfo {
                    name: p.project_name.clone(),
                    path: p.project_path.clone(),
                    _track_count: 0,
                }).collect();
                self.stats.project_count = self.projects.len();
            }

	            let memory_limit = 200usize;
	            let memories_res = if self.memory_query.trim().is_empty() {
	                svc.list_memories(memory_limit)
	            } else {
	                svc.search_memories(self.memory_query.trim(), memory_limit)
	            };
	            if let Ok(memories) = memories_res {
	                self.memories = memories.iter().map(|m| MemoryInfo {
	                    _id: m.id,
	                    content: m.content.clone(),
	                    category: m.category.to_string(),
	                }).collect();
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
                    leindex_analyzers::memory::models::SessionStatus::Running
                } else {
                    leindex_analyzers::memory::models::SessionStatus::Terminated
                };

                if session.status != new_status {
                    session.status = new_status;
                    // Best-effort: persist status transitions so restart/resume logic is consistent.
                    let _ = svc.update_session_status(&session.session_id, new_status);
                }
            }

            self.refresh_session_entries();
            self.refresh_dash_session_entries();
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
                for session in self.sessions.iter().filter(|s| s.group_path.as_deref() == Some(&group.path)) {
                    entries.push(SessionEntry::Session(session.clone()));
                }
            }
        }

        // Add Uncategorized as a selectable Group if sessions exist
        let has_uncategorized = self.sessions.iter().any(|s| s.group_path.is_none());
        if has_uncategorized {
            let uncategorized_group = leindex_analyzers::memory::models::SessionGroup {
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
                for s in recent_sessions
                    .iter()
                    .filter(|rs| rs.group_path.clone().unwrap_or_else(|| "uncategorized".to_string()) == group_key)
                {
                    entries.push(DashSessionEntry::Session(s.clone()));
                }
            }
        }

        self.dash_session_entries = entries;
        if let Some(session_id) = selected_session_id {
            if let Some(idx) = self.dash_session_entries.iter().position(|e| {
                matches!(e, DashSessionEntry::Session(s) if s.session_id == session_id)
            }) {
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
    fn refresh_lsp_status(&mut self) {
        self.refresh_lsp_status_impl(false);
    }

    /// Refresh LSP status cache from Turso database (internal implementation)
    ///
    /// Sets a flag to trigger async refresh in the main event loop.
    /// This avoids the Tokio panic when calling async from sync context.
    fn refresh_lsp_status_impl(&mut self, force: bool) {
        // Only refresh every 2 seconds to avoid excessive async calls (unless forced)
        if !force && self.last_lsp_refresh.elapsed() < std::time::Duration::from_secs(2) {
            return;
        }
        self.last_lsp_refresh = Instant::now();
        self.pending_lsp_refresh = true;
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
                Err(e) => {
                    // Log error but continue with other sessions
                    eprintln!("Failed to get LSP states for session {}: {}", session_id, e);
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
            "ruff-lsp" => Some(LspType::Python),
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

        let Some(storage) = self.storage_backend.clone() else {
            self.status_message = "Storage backend not available".to_string();
            return;
        };

        let session_id = session_id.to_string();
        let lsp_name = lsp_name.to_string();

        // Spawn the operation in the background
        tokio::spawn(async move {
            let lsp_manager = leindex_analyzers::memory::lsp_manager::LspManager::new((*storage).clone());

            let result = match status {
                LspStatus::Stopped | LspStatus::Error => {
                    // Start the LSP
                    lsp_manager.start_lsp(&session_id, lsp_type, None).await
                }
                LspStatus::Running | LspStatus::Starting => {
                    // Stop the LSP
                    lsp_manager.stop_lsp(&session_id, lsp_type).await
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

        let Some(storage) = self.storage_backend.clone() else {
            self.status_message = "Storage backend not available".to_string();
            return;
        };

        let session_id = session_id.to_string();
        let lsp_name = lsp_name.to_string();

        // Spawn the operation in the background
        tokio::spawn(async move {
            let lsp_manager = leindex_analyzers::memory::lsp_manager::LspManager::new((*storage).clone());
            lsp_manager.restart_lsp(&session_id, lsp_type).await
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
        let safe_session_id = if safe_session_id.is_empty() || safe_session_id.len() != session_id.len() {
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
            lsp_name, session_id, safe_lsp_name, safe_session_id, safe_session_id, safe_lsp_name, safe_session_id
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
        if app.tab_index == 1 && app.last_preview_refresh.elapsed() >= std::time::Duration::from_millis(200) {
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
                                    InputMode::NewSessionTitle => app.input_mode = InputMode::NewSessionPath,
                                    InputMode::NewSessionPath => app.input_mode = InputMode::NewSessionTool,
                                    InputMode::NewSessionTool => {
                                        app.is_spawning = true;
                                        app.status_message = format!("Spawning {} session...", app.new_session_tool);
                                        // let _ = terminal.draw(|frame| ui(frame, \u0026mut app));

                                        if let Some(svc) = service.as_ref() {
                                            let manager = match leindex_analyzers::memory::session_manager::SessionManager::new(svc.clone()) {
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
                                                    app.sessions.push(session.clone());
                                                    app.refresh_session_entries();
                                                    app.refresh_dash_session_entries();
                                                    app.status_message = format!("Session '{}' created. Press Enter on Sessions tab to attach.", session.title);
                                                    app.tab_index = 1;
                                                    let new_idx = app.session_entries.iter().position(|e| {
                                                        if let SessionEntry::Session(s) = e {
                                                            s.session_id == session.session_id
                                                        } else {
                                                            false
                                                        }
                                                    }).unwrap_or(0);
                                                    app.session_state.select(Some(new_idx));
                                                }
                                                Err(e) => {
                                                    app.status_message = format!("Error: {}", e);
                                                    let _ = terminal.draw(|frame| ui(frame, &mut app));
                                                    std::thread::sleep(std::time::Duration::from_secs(2));
                                                }
                                            }
                                        } else {
                                            app.status_message = "Error: Memory service not available".to_string();
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
                                                app.status_message = format!("Attaching to '{}'... (Ctrl+B d to detach)", session.title);
                                                let _ = terminal.draw(|frame| ui(frame, &mut app));
                                                let _ = suspend_fullscreen_app(terminal);
                                                let res = TmuxMultiplexer::attach(&session.session_id);
                                                let _ = resume_fullscreen_app(terminal);
                                                let _ = terminal.clear();
                                                app.status_message = match res {
                                                    Ok(()) => format!("Returned from '{}'", session.title),
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
                                            app.status_message = "Group name cannot be empty".to_string();
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
                                            let new_path =
                                                format!("/{}", clean_name.to_lowercase().replace(' ', "_"));

                                            let group = leindex_analyzers::memory::models::SessionGroup {
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
                                                let _ = svc.update_group_category(&new_path, category);
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

                                                    let _ = svc.update_group_category(&new_path, category);
                                                    let _ = svc.update_group_expansion(&new_path, true);
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
                                                let _ = svc.update_group_expansion(group_path, true);
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
                                                if let Some(orig) = app.sessions.iter().find(|s| s.session_id == id) {
                                                    let manager = match leindex_analyzers::memory::session_manager::SessionManager::new(svc.clone()) {
                                                        Ok(m) => m,
                                                        Err(e) => {
                                                            app.status_message = format!("Failed to create session manager: {}", e);
                                                            app.input_mode = InputMode::Normal;
                                                            continue;
                                                        }
                                                    };
                                                    let _ = manager.fork_session(&id, &app.rename_buffer, orig);
                                                    app.status_message = format!("Session forked as {}", app.rename_buffer);
                                                    if let Ok(sessions) = svc.list_sessions() { app.sessions = sessions; }
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
                                                let manager = match leindex_analyzers::memory::session_manager::SessionManager::new(svc.clone()) {
                                                    Ok(m) => m,
                                                    Err(e) => {
                                                        app.status_message = format!("Failed to create session manager: {}", e);
                                                        continue;
                                                    }
                                                };
                                                match manager.kill_session(&id) {
                                                    Ok(()) => {
                                                        if app.input_mode == InputMode::DeleteConfirm {
                                                            let _ = svc.delete_session(&id);
                                                            app.status_message = "Session deleted".to_string();
                                                        } else {
                                                            app.status_message = "Session killed".to_string();
                                                        }
                                                        if let Ok(sessions) = svc.list_sessions() {
                                                            app.sessions = sessions;
                                                        }
                                                    }
                                                    Err(e) => {
                                                        app.status_message = format!("Kill failed: {}", e);
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
                                            let tokens: Vec<&str> = input.split_whitespace().collect();
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
                                                    let drain = app.analysis_history.len() - MAX_LINES;
                                                    app.analysis_history.drain(0..drain);
                                                }
                                            };

                                            let parse_phase_opts = || {
                                                let mut opts =
                                                    leindex_analyzers::five_phase::PhaseOptions::new(
                                                        std::path::PathBuf::from("."),
                                                    );

                                                let mut path_set = false;
                                                let mut i = 1usize;
                                                while i < tokens.len() {
                                                    let t = tokens[i];
                                                    match t {
                                                        "--mode" | "-m" => {
                                                            if let Some(v) = tokens.get(i + 1) {
                                                                opts.mode = leindex_analyzers::token_format::FormatMode::from_str(v);
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
                                                                    leindex_analyzers::token_format::FormatMode::from_str(v);
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
                                                            opts.mode = leindex_analyzers::token_format::FormatMode::Ultra
                                                        }
                                                        "balanced" | "b" => {
                                                            opts.mode =
                                                                leindex_analyzers::token_format::FormatMode::Balanced
                                                        }
                                                        "verbose" | "v" => {
                                                            opts.mode =
                                                                leindex_analyzers::token_format::FormatMode::Verbose
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
                                                    match leindex_analyzers::five_phase::phase1_structural_scan(&opts)
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
                                                    match leindex_analyzers::five_phase::phase2_dependency_map(&opts) {
                                                        Ok(out) => push_block(&out),
                                                        Err(e) => push_block(&format!(
                                                            "Error running /phase2: {}",
                                                            e
                                                        )),
                                                    }
                                                }
                                                "/phase3" | "/p3" => {
                                                    let opts = parse_phase_opts();
                                                    match leindex_analyzers::five_phase::phase3_logic_flow(&opts) {
                                                        Ok(out) => push_block(&out),
                                                        Err(e) => push_block(&format!(
                                                            "Error running /phase3: {}",
                                                            e
                                                        )),
                                                    }
                                                }
                                                "/phase4" | "/p4" => {
                                                    let opts = parse_phase_opts();
                                                    match leindex_analyzers::five_phase::phase4_critical_path(&opts) {
                                                        Ok(out) => push_block(&out),
                                                        Err(e) => push_block(&format!(
                                                            "Error running /phase4: {}",
                                                            e
                                                        )),
                                                    }
                                                }
                                                "/phase5" | "/p5" => {
                                                    let opts = parse_phase_opts();
                                                    match leindex_analyzers::five_phase::phase5_optimization_report(&opts) {
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
                                            McpOption::StartStop => {
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

                                                if server.status == McpStatus::Running {
                                                    if let Err(e) = pool.stop_server(&name).await {
                                                        app.status_message =
                                                            format!("Stop failed: {}", e);
                                                    } else {
                                                        app.status_message =
                                                            format!("Stopped MCP '{}'", name);
                                                    }
                                                } else {
                                                    match pool.start_server_record(&server).await {
                                                        Ok(socket) => {
                                                            app.status_message = format!(
                                                                "Started MCP '{}' at {}",
                                                                name, socket
                                                            );
                                                        }
                                                        Err(e) => {
                                                            app.status_message =
                                                                format!("Start failed: {}", e);
                                                        }
                                                    }
                                                }
                                            }
                                            McpOption::Pause => {
                                                app.status_message =
                                                    format!("MCP '{}' pause not implemented", name);
                                            }
                                            McpOption::Logs => {
                                                let log_path = McpPool::log_path_for(&name);
                                                let content =
                                                    std::fs::read_to_string(&log_path).unwrap_or_default();
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
                                                    Ok(n) => app.status_message = format!(
                                                        "Discovered {} MCP server(s) from system configs",
                                                        n
                                                    ),
                                                    Err(e) => app.status_message =
                                                        format!("Discovery failed: {}", e),
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
                                            McpOption::Install => {
                                                let discovered = svc
                                                    .sync_mcp_servers_from_system()
                                                    .unwrap_or(0);
                                                if let Some(pool) = app.mcp_pool.clone() {
                                                    let _ = pool.start_all_from_db().await;
                                                }
                                                app.status_message = format!(
                                                    "MCP pool synced ({} discovered)",
                                                    discovered
                                                );
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
                                        app.status_message = format!("Editor set to '{}'", app.config.editor);
                                    }
                                    InputMode::SettingsInstallPath => {
                                        app.config.install_path = app.rename_buffer.clone();
                                        let _ = app.config.save();
                                        app.input_mode = InputMode::Normal;
                                        app.status_message = format!("Install path set to '{}'", app.config.install_path);
                                    }
                                    InputMode::SettingsMenu => {
                                        let Some(kind) = app.settings_menu_kind else {
                                            app.input_mode = InputMode::Normal;
                                            continue;
                                        };
                                        let idx = app.settings_menu_state.selected().unwrap_or(0);
                                        let Some((id, _label)) = app.settings_menu_items.get(idx).cloned() else {
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
                                                app.status_message =
                                                    format!("Editor set to '{}'", app.config.editor);
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
                                    InputMode::NewGroupCategory => {
                                        if let Some(svc) = service.as_ref() {
                                            let clean_name = app.rename_buffer.trim();
                                            if !clean_name.is_empty() {
                                                let path = format!("/{}", clean_name.to_lowercase().replace(' ', "_"));
                                                let category = if app.new_group_category.trim().is_empty() { None } else { Some(app.new_group_category.clone()) };
                                                
                                                // Ensure group exists
                                                let group = leindex_analyzers::memory::models::SessionGroup {
                                                    id: 0,
                                                    name: clean_name.to_string(),
                                                    path: path.clone(),
                                                    category: category,
                                                    is_expanded: true,
                                                    sort_order: 0,
                                                    parent_id: None,
                                                };
                                                let _ = svc.get_or_create_session_group(group);
                                                app.status_message = format!("Group '{}' ready", clean_name);
                                                if let Ok(groups) = svc.list_session_groups() { app.groups = groups; }
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
                                                let target = if app.rename_buffer.is_empty() { None } else { Some(app.rename_buffer.clone()) };
                                                let _ = svc.update_session_group(&id, target);
                                                app.status_message = format!("Session moved to group");
                                                if let Ok(sessions) = svc.list_sessions() { app.sessions = sessions; }
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
                                                    if let Some(id) = app.target_session_id.clone() {
                                                        let manager = match leindex_analyzers::memory::session_manager::SessionManager::new(svc.clone()) {
                                                            Ok(m) => m,
                                                            Err(e) => {
                                                                app.status_message = format!("Failed to create session manager: {}", e);
                                                                app.input_mode = InputMode::Normal;
                                                                continue;
                                                            }
                                                        };
                                                        let _ = manager.rename_session(&id, &app.rename_buffer);
                                                        if let Ok(sessions) = svc.list_sessions() { app.sessions = sessions; }
                                                        app.refresh_session_entries();
                                                        app.refresh_dash_session_entries();
                                                        app.status_message = "Session renamed".to_string();
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
                                        app.status_message = format!("Initializing project '{}' at {} with tool {}...", name, path, tool);
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
	                                        app.status_message = format!("Creating {} track: {}...", if is_master { "master" } else { "direct" }, title);
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
	                                    _ => {}
	                                }
	                                app.input_mode = InputMode::Normal;
	                            }
	                            KeyCode::Backspace => {
	                                match app.input_mode {
                                    InputMode::NewSessionTitle => { app.new_session_title.pop(); }
                                    InputMode::NewSessionPath => { app.new_session_path.pop(); }
                                    InputMode::RenameGroup | InputMode::ForkSession | InputMode::NewGroupTitle | InputMode::SettingsEditor | InputMode::SettingsInstallPath => {
                                        app.rename_buffer.pop();
                                    }
                                    InputMode::RenameGroupCategory | InputMode::NewGroupCategory => {
                                        app.new_group_category.pop();
                                    }
                                    InputMode::KillConfirm => {
                                        app.target_session_id = None;
                                        app.input_mode = InputMode::Normal;
                                    }
	                                    InputMode::AnalysisPrompt => {
	                                        app.analysis_input.pop();
	                                    }
	                                    InputMode::MemorySearch => {
	                                        app.memory_query.pop();
	                                    }
	                                    InputMode::NewProjectName => { app.new_project_name.pop(); }
                                    InputMode::NewProjectPath => { app.new_project_path.pop(); }
                                    InputMode::NewProjectTool => { app.new_project_tool.pop(); }
                                    InputMode::NewTrackTitle => { app.new_track_title.pop(); }
                                    InputMode::SessionHub => {
                                        match app.hub_focus {
                                            HubFocus::Rename => { app.rename_buffer.pop(); }
                                            HubFocus::Search => { app.hub_search_buffer.pop(); }
                                            _ => {}
                                        }
                                    }
                                    _ => {}
                                }
                            }
	                            KeyCode::Char(c) => {
                                if c == '\n' || c == '\r' {
                                    continue;
                                }
	                                match app.input_mode {
                                    InputMode::NewSessionTitle => app.new_session_title.push(c),
                                    InputMode::NewSessionPath => app.new_session_path.push(c),
                                    InputMode::NewSessionTool => {
                                        // Cycle tools
                                        let tools = vec!["claude", "gemini", "shell", "codex", "opencode", "amp"];
                                        if let Some(pos) = tools.iter().position(|&t| t == app.new_session_tool) {
                                            app.new_session_tool = tools[(pos + 1) % tools.len()].to_string();
                                        }
                                    }
                                    InputMode::RenameGroup | InputMode::ForkSession | InputMode::NewGroupTitle | InputMode::MoveToGroup | InputMode::SettingsEditor | InputMode::SettingsInstallPath => app.rename_buffer.push(c),
                                    InputMode::RenameGroupCategory | InputMode::NewGroupCategory => app.new_group_category.push(c),
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
	                                    InputMode::KillConfirm | InputMode::DeleteConfirm => {
                                        if c == 'y' || c == 'Y' {
                                            if let Some(svc) = service.as_ref() {
                                                if let Some(id) = app.target_session_id.take() {
                                                    let manager = match leindex_analyzers::memory::session_manager::SessionManager::new(svc.clone()) {
                                                        Ok(m) => m,
                                                        Err(e) => {
                                                            app.status_message = format!("Failed to create session manager: {}", e);
                                                            continue;
                                                        }
                                                    };
                                                    match manager.kill_session(&id) {
                                                        Ok(()) => {
                                                            if app.input_mode == InputMode::DeleteConfirm {
                                                                let _ = svc.delete_session(&id);
                                                                app.status_message = "Session deleted".to_string();
                                                            } else {
                                                                app.status_message = "Session killed".to_string();
                                                            }
                                                            if let Ok(sessions) = svc.list_sessions() {
                                                                app.sessions = sessions;
                                                            }
                                                        }
                                                        Err(e) => {
                                                            app.status_message = format!("Kill failed: {}", e);
                                                        }
                                                    }
                                                }
                                                // Group delete logic
                                                if let Some(path) = app.target_group_path.take() {
                                                    let _ = svc.delete_group(&path);
                                                    app.status_message = format!("Group deleted");
                                                    if let Ok(groups) = svc.list_session_groups() { app.groups = groups; }
                                                    if let Ok(sessions) = svc.list_sessions() { app.sessions = sessions; }
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
                            KeyCode::Tab => {
                                match app.input_mode {
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
                                    _ => {}
                                }
                            }
                            KeyCode::BackTab => {
                                match app.input_mode {
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
                                    _ => {}
                                }
                            }
	                            KeyCode::Down => {
	                                if app.input_mode == InputMode::SessionSwitcher {
	                                    let i = match app.switcher_state.selected() {
	                                        Some(i) => if i >= app.sessions.len().saturating_sub(1) { 0 } else { i + 1 },
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
	                                        Some(i) => if i >= len.saturating_sub(1) { 0 } else { i + 1 },
	                                        None => 0,
	                                    };
	                                    app.settings_menu_state.select(Some(i));
	                                } else if app.input_mode == InputMode::McpMenu {
	                                    app.mcp_menu_option = match app.mcp_menu_option {
	                                        McpOption::StartStop => McpOption::Pause,
	                                        McpOption::Pause => McpOption::Logs,
                                        McpOption::Logs => McpOption::Add,
                                        McpOption::Add => McpOption::Remove,
                                        McpOption::Remove => McpOption::Install,
                                        McpOption::Install => McpOption::StartStop,
                                    };
                                } else if app.preview_focused {
                                    app.preview_scroll = app.preview_scroll.saturating_add(1);
                                 } else if app.tab_index == 0 { // Dashboard
                                     match app.dash_focus {
                                         DashFocus::Sessions => {
                                             app.dash_select_next_session();
                                         }
                                         DashFocus::Mcp => {
                                             let i = match app.mcp_state.selected() {
                                                 Some(i) => if i >= app.mcp_servers.len().saturating_sub(1) { 0 } else { i + 1 },
                                                 None => 0,
                                             };
                                             app.mcp_state.select(Some(i));
                                         }
                                         DashFocus::Tabs => {}
                                     }
                                 } else if app.tab_index == 2 { // Projects
                                    if app.preview_focused {
                                        app.project_explorer_selected = (app.project_explorer_selected + 1) % app.explorer_items.len().max(1);
                                    } else {
                                        let i = match app.project_state.selected() {
                                            Some(i) => if i >= app.projects.len().saturating_sub(1) { 0 } else { i + 1 },
                                            None => 0,
                                        };
                                        app.project_state.select(Some(i));
                                        app.project_explorer_path = None;
                                        app.project_explorer_selected = 0;
                                    }
	                                 } else if app.tab_index == 1 { // Sessions Tab
	                                    let i = match app.session_state.selected() {
	                                        Some(i) => if i >= app.session_entries.len().saturating_sub(1) { 0 } else { i + 1 },
	                                        None => 0,
	                                    };
	                                    app.session_state.select(Some(i));
	                                } else if app.tab_index == 4 { // Memory
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
	                                } else if app.tab_index == 6 { // Settings
	                                    app.settings_option = match app.settings_option {
	                                        SettingsOption::Editor => SettingsOption::Theme,
	                                        SettingsOption::Theme => SettingsOption::InstallPath,
	                                        SettingsOption::InstallPath => SettingsOption::Save,
	                                        SettingsOption::Save => SettingsOption::Editor,
	                                    };
	                                }
                                app.scroll = app.scroll.saturating_add(1);
                            }
	                            KeyCode::Up => {
	                                if app.input_mode == InputMode::McpMenu {
	                                    app.mcp_menu_option = match app.mcp_menu_option {
	                                        McpOption::StartStop => McpOption::Install,
	                                        McpOption::Pause => McpOption::StartStop,
	                                        McpOption::Logs => McpOption::Pause,
	                                        McpOption::Add => McpOption::Logs,
	                                        McpOption::Remove => McpOption::Add,
	                                        McpOption::Install => McpOption::Remove,
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
	                                        Some(i) => if i == 0 { len.saturating_sub(1) } else { i - 1 },
	                                        None => 0,
	                                    };
	                                    app.settings_menu_state.select(Some(i));
	                                } else if app.preview_focused {
	                                    app.preview_scroll = app.preview_scroll.saturating_sub(1);
	                                 } else if app.tab_index == 0 { // Dashboard
	                                     match app.dash_focus {
                                         DashFocus::Sessions => {
                                             app.dash_select_prev_session();
                                         }
                                         DashFocus::Mcp => {
                                             let i = match app.mcp_state.selected() {
                                                 Some(i) => if i == 0 { app.mcp_servers.len().saturating_sub(1) } else { i - 1 },
                                                 None => 0,
                                             };
                                             app.mcp_state.select(Some(i));
                                         }
                                         DashFocus::Tabs => {}
                                     }
                                } else if app.tab_index == 2 { // Projects Tab
                                    if app.preview_focused {
                                        app.project_explorer_selected = if app.project_explorer_selected == 0 { app.explorer_items.len().saturating_sub(1) } else { app.project_explorer_selected - 1 };
                                    } else {
                                        let i = match app.project_state.selected() {
                                            Some(i) => if i == 0 { app.projects.len().saturating_sub(1) } else { i - 1 },
                                            None => 0,
                                        };
                                        app.project_state.select(Some(i));
                                        app.project_explorer_path = None;
                                        app.project_explorer_selected = 0;
                                    }
	                                } else if app.tab_index == 1 { // Sessions Tab
	                                    let i = match app.session_state.selected() {
	                                        Some(i) => if i == 0 { app.session_entries.len().saturating_sub(1) } else { i - 1 },
	                                        None => 0,
	                                    };
	                                    app.session_state.select(Some(i));
	                                } else if app.tab_index == 4 { // Memory
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
	                                } else if app.tab_index == 6 { // Settings
	                                    app.settings_option = match app.settings_option {
	                                        SettingsOption::Editor => SettingsOption::Save,
	                                        SettingsOption::Theme => SettingsOption::Editor,
	                                        SettingsOption::InstallPath => SettingsOption::Theme,
	                                        SettingsOption::Save => SettingsOption::InstallPath,
	                                    };
	                                }
                                app.scroll = app.scroll.saturating_sub(1);
                            }
                            _ => {}
                        }

                    } else if app.show_help {
                        let max_scroll = build_help_text(&app)
                            .len()
                            .saturating_sub(1) as u16;

                        match key.code {
                            KeyCode::Esc | KeyCode::Char('/') | KeyCode::Char('?') => {
                                app.show_help = false;
                            }
                            KeyCode::Up => app.help_scroll = app.help_scroll.saturating_sub(1),
                            KeyCode::Down => app.help_scroll = app.help_scroll.saturating_add(1),
                            KeyCode::PageUp => app.help_scroll = app.help_scroll.saturating_sub(10),
                            KeyCode::PageDown => app.help_scroll = app.help_scroll.saturating_add(10),
                            KeyCode::Home => app.help_scroll = 0,
                            KeyCode::End => app.help_scroll = max_scroll,
                            _ => {}
                        }
                        app.help_scroll = app.help_scroll.min(max_scroll);
                    } else {
	                        match (key.modifiers, key.code) {
	                            (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
	                                if app.tab_index == 4 {
	                                    app.input_mode = InputMode::MemorySearch;
	                                }
	                            }
	                            (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
	                                if app.tab_index == 4 {
	                                    app.memory_query.clear();
	                                    app.refresh_from_service(&service);
	                                }
	                            }
	                            (KeyModifiers::ALT, KeyCode::Char('p')) => {
	                                if app.tab_index == 1 {
	                                    app.preview_focused = !app.preview_focused;
	                                    app.status_message = if app.preview_focused { "Preview focused. Scroll with Arrows/PgUp/PgDn." } else { "List focused." }.to_string();
	                                }
	                            }
	                            (KeyModifiers::ALT, KeyCode::Up) | (KeyModifiers::ALT, KeyCode::Down) => {
	                                if app.tab_index == 1 {
	                                    let Some(svc) = service.as_ref() else {
	                                        app.status_message = "Error: Memory service not available".to_string();
	                                        continue;
	                                    };

	                                    let delta: i32 = if matches!(key.code, KeyCode::Up) { -1 } else { 1 };
	                                    let Some(selected) = app.session_state.selected() else { continue; };
	                                    let Some(entry) = app.session_entries.get(selected).cloned() else { continue; };

	                                    match entry {
	                                        SessionEntry::Group(g) => {
	                                            if g.path == "uncategorized" {
	                                                app.status_message = "Cannot reorder [Uncategorized]".to_string();
	                                                continue;
	                                            }

	                                            let mut paths: Vec<String> =
	                                                app.groups.iter().map(|gg| gg.path.clone()).collect();
	                                            let Some(pos) = paths.iter().position(|p| p == &g.path) else {
	                                                continue;
	                                            };

	                                            let new_pos = if delta < 0 {
	                                                pos.checked_sub(1)
	                                            } else if pos + 1 < paths.len() {
	                                                Some(pos + 1)
	                                            } else {
	                                                None
	                                            };
	                                            let Some(new_pos) = new_pos else { continue; };

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

	                                            let Some(pos) = ids.iter().position(|id| id == &s.session_id) else {
	                                                continue;
	                                            };
	                                            let new_pos = if delta < 0 {
	                                                pos.checked_sub(1)
	                                            } else if pos + 1 < ids.len() {
	                                                Some(pos + 1)
	                                            } else {
	                                                None
	                                            };
	                                            let Some(new_pos) = new_pos else { continue; };

	                                            ids.swap(pos, new_pos);
	                                            if svc
	                                                .reorder_sessions_in_group(group_key.as_deref(), &ids)
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
	                            (_, KeyCode::Char('p')) => {
	                                if app.tab_index == 2 { // Projects
	                                    app.input_mode = InputMode::NewProjectName;
	                                    app.new_project_name.clear();
                                    app.new_project_path = std::env::current_dir().unwrap_or_default().to_string_lossy().to_string();
                                    app.new_project_tool.clear();
                                }
                            }
                            (_, KeyCode::Char('t')) => {
                                if app.tab_index == 2 { // Projects
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
                            (_, KeyCode::Esc) => {
                                if app.project_view_open {
                                    app.project_view_open = false;
                                }
                            }
	                            (_, KeyCode::Char('r')) => {
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
	                                } else if app.tab_index == 4 {
	                                    if let Some(svc) = service.as_ref() {
	                                        match svc.sync_memories_from_system() {
	                                            Ok(n) => {
	                                                app.status_message =
	                                                    format!("Memory refresh imported {} record(s)", n)
	                                            }
	                                            Err(e) => {
	                                                app.status_message =
	                                                    format!("Memory refresh failed: {}", e)
	                                            }
	                                        }
	                                        app.refresh_from_service(&service);
	                                    }
	                                } else if app.tab_index == 5 { // LSPs tab
	                                    app.refresh_lsp_status();
	                                    app.status_message = "LSP status refreshed".to_string();
	                                }
	                            }
                             (_, KeyCode::Char('k')) => {
                                 if app.tab_index == 1 {
                                     if let Some(i) = app.session_state.selected() {
                                         if let Some(SessionEntry::Session(s)) = app.session_entries.get(i) {
                                             app.target_session_id = Some(s.session_id.clone());
                                             app.input_mode = InputMode::KillConfirm;
                                         }
                                     }
                                 } else if app.tab_index == 0 && app.dash_focus == DashFocus::Sessions {
                                     if let Some(session_id) = app
                                         .dash_selected_session()
                                         .map(|s| s.session_id.clone())
                                     {
                                         app.target_session_id = Some(session_id);
                                         app.input_mode = InputMode::KillConfirm;
                                     }
                                 }
                             }
	                            (_, KeyCode::Char('f')) => {
	                                if app.tab_index == 1 {
	                                    if let Some(i) = app.session_state.selected() {
	                                        if let Some(SessionEntry::Session(s)) = app.session_entries.get(i) {
	                                            app.target_session_id = Some(s.session_id.clone());
	                                            app.rename_buffer = format!("{}-fork", s.title);
	                                            app.input_mode = InputMode::ForkSession;
	                                        }
	                                    }
	                                }
	                            }
	                            (_, KeyCode::Char('l')) => {
	                                if app.tab_index == 5 { // LSPs tab - view logs
	                                    if let Some((session_id, lsp_name, _status)) = app.get_selected_lsp() {
	                                        app.read_lsp_logs(&session_id, &lsp_name);
	                                        app.input_mode = InputMode::McpLogs; // Reuse the existing log viewer
	                                        app.status_message = format!("Viewing logs for '{}' (press Esc to close)", lsp_name);
	                                    } else {
	                                        app.status_message = "No LSP selected".to_string();
	                                    }
	                                }
	                            }
	                            (_, KeyCode::Char('u') | KeyCode::Char('U')) => {
	                                if app.tab_index == 1 {
	                                    let Some(i) = app.session_state.selected() else { continue; };
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

	                                    let manager = match leindex_analyzers::memory::session_manager::SessionManager::new(
	                                        svc.clone(),
	                                    ) {
	                                        Ok(m) => m,
	                                        Err(e) => {
	                                            app.status_message =
	                                                format!("Failed to create session manager: {}", e);
	                                            app.is_spawning = false;
	                                            continue;
	                                        }
	                                    };

	                                    let res = manager.restore_session(
	                                        &s,
	                                        leindex_analyzers::memory::session_manager::SessionRestoreMode::Resume,
	                                    );
	                                    app.is_spawning = false;
	                                    app.refresh_from_service(&service);

	                                    match res {
	                                        Ok(()) => {
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
	                                        Err(e) => {
	                                            app.status_message = format!("Resume failed: {}", e);
	                                        }
	                                    }
	                                }
	                            }
	                             (KeyModifiers::ALT, KeyCode::Char('d')) | (KeyModifiers::ALT, KeyCode::Char('D')) | (KeyModifiers::NONE, KeyCode::Char('d')) => {
	                                 if app.tab_index == 1 {
	                                     if let Some(i) = app.session_state.selected() {
	                                         if let Some(entry) = app.session_entries.get(i) {
	                                             match entry {
                                                 SessionEntry::Session(s) => {
                                                     app.target_session_id = Some(s.session_id.clone());
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
                                 } else if app.tab_index == 0 && app.dash_focus == DashFocus::Sessions {
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
                                    app.status_message = "Project deletion via TUI coming soon in v2.1".to_string();
                                }
                            }
                            (_, KeyCode::Tab) => {
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
                                    app.tab_index = (app.tab_index + 1) % 7;
                                    app.preview_focused = false;
                                }
                            }
                            (_, KeyCode::BackTab) => {
                                if app.tab_index == 0 {
                                    match app.dash_focus {
                                        DashFocus::Sessions => {
                                            app.tab_index = 6;
                                            app.dash_focus = DashFocus::Sessions;
                                        }
                                        DashFocus::Mcp => app.dash_focus = DashFocus::Sessions,
                                        DashFocus::Tabs => app.dash_focus = DashFocus::Mcp,
                                    };
                                } else {
                                    app.tab_index = if app.tab_index == 0 { 6 } else { app.tab_index - 1 };
                                    app.preview_focused = false;
                                }
                            }
                            (KeyModifiers::ALT, KeyCode::Char('o')) => {
                                app.tab_index = if app.tab_index == 0 { 6 } else { app.tab_index - 1 };
                                app.preview_focused = false;
                            }
                            (KeyModifiers::ALT, KeyCode::Char('i')) => {
                                app.tab_index = (app.tab_index + 1) % 7;
                                app.preview_focused = false;
                            }
                             (_, KeyCode::Down) => {
                                if app.input_mode == InputMode::McpMenu {
                                    app.mcp_menu_option = match app.mcp_menu_option {
                                        McpOption::StartStop => McpOption::Pause,
                                        McpOption::Pause => McpOption::Logs,
                                        McpOption::Logs => McpOption::Add,
                                        McpOption::Add => McpOption::Remove,
                                        McpOption::Remove => McpOption::Install,
                                        McpOption::Install => McpOption::StartStop,
                                    };
                                } else if app.preview_focused {
                                    app.preview_scroll = app.preview_scroll.saturating_add(1);
                                 } else if app.tab_index == 0 { // Dashboard
                                     match app.dash_focus {
                                         DashFocus::Sessions => {
                                             app.dash_select_next_session();
                                         }
                                         DashFocus::Mcp => {
                                             let i = match app.mcp_state.selected() {
                                                 Some(i) => if i >= app.mcp_servers.len().saturating_sub(1) { 0 } else { i + 1 },
                                                 None => 0,
                                             };
                                             app.mcp_state.select(Some(i));
                                         }
                                         DashFocus::Tabs => {}
                                     }
                                 } else if app.tab_index == 2 { // Projects
                                    if app.preview_focused {
                                        app.project_explorer_selected = (app.project_explorer_selected + 1) % app.explorer_items.len().max(1);
                                    } else {
                                        let i = match app.project_state.selected() {
                                            Some(i) => if i >= app.projects.len().saturating_sub(1) { 0 } else { i + 1 },
                                            None => 0,
                                        };
                                        app.project_state.select(Some(i));
                                        app.project_explorer_path = None;
                                        app.project_explorer_selected = 0;
                                    }
                                } else if app.tab_index == 1 { // Sessions Tab
                                    let i = match app.session_state.selected() {
                                        Some(i) => if i >= app.session_entries.len().saturating_sub(1) { 0 } else { i + 1 },
                                        None => 0,
                                    };
                                    app.session_state.select(Some(i));
                                } else if app.tab_index == 4 { // Memory
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
                                } else if app.tab_index == 5 { // LSPs
                                    let i = match app.lsp_state.selected() {
                                        Some(i) => {
                                            if i >= app.lsp_status_cache.values().map(|v| v.len()).sum::<usize>().saturating_sub(1) {
                                                0
                                            } else {
                                                i + 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    app.lsp_state.select(Some(i));
                                } else if app.tab_index == 6 { // Settings
                                    app.settings_option = match app.settings_option {
                                        SettingsOption::Editor => SettingsOption::Theme,
                                        SettingsOption::Theme => SettingsOption::InstallPath,
                                        SettingsOption::InstallPath => SettingsOption::Save,
                                        SettingsOption::Save => SettingsOption::Editor,
                                    };
                                }
                                app.scroll = app.scroll.saturating_add(1);
                            }
                            (_, KeyCode::Up) => {
                                if app.input_mode == InputMode::McpMenu {
                                    app.mcp_menu_option = match app.mcp_menu_option {
                                        McpOption::StartStop => McpOption::Install,
                                        McpOption::Pause => McpOption::StartStop,
                                        McpOption::Logs => McpOption::Pause,
                                        McpOption::Add => McpOption::Logs,
                                        McpOption::Remove => McpOption::Add,
                                        McpOption::Install => McpOption::Remove,
                                    };
                                } else if app.preview_focused {
                                    app.preview_scroll = app.preview_scroll.saturating_sub(1);
                                } else if app.tab_index == 0 { // Dashboard
                                    match app.dash_focus {
                                        DashFocus::Sessions => {
                                            app.dash_select_prev_session();
                                        }
                                        DashFocus::Mcp => {
                                            let i = match app.mcp_state.selected() {
                                                Some(i) => if i == 0 { app.mcp_servers.len().saturating_sub(1) } else { i - 1 },
                                                None => 0,
                                            };
                                            app.mcp_state.select(Some(i));
                                        }
                                         DashFocus::Tabs => {},
                                    }
                                } else if app.tab_index == 2 { // Projects Tab
                                    if app.preview_focused {
                                        app.project_explorer_selected = if app.project_explorer_selected == 0 { app.explorer_items.len().saturating_sub(1) } else { app.project_explorer_selected - 1 };
                                    } else {
                                        let i = match app.project_state.selected() {
                                            Some(i) => if i == 0 { app.projects.len().saturating_sub(1) } else { i - 1 },
                                            None => 0,
                                        };
                                        app.project_state.select(Some(i));
                                        app.project_explorer_path = None;
                                        app.project_explorer_selected = 0;
                                    }
	                                } else if app.tab_index == 1 { // Sessions Tab
	                                    let i = match app.session_state.selected() {
	                                        Some(i) => if i == 0 { app.session_entries.len().saturating_sub(1) } else { i - 1 },
	                                        None => 0,
	                                    };
	                                    app.session_state.select(Some(i));
	                            } else if app.tab_index == 4 { // Memory
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
	                            } else if app.tab_index == 5 { // LSPs
	                                    let i = match app.lsp_state.selected() {
	                                        Some(i) => {
	                                            let total_lsps = app.lsp_status_cache.values().map(|v| v.len()).sum::<usize>();
	                                            if i == 0 {
	                                                total_lsps.saturating_sub(1)
	                                            } else {
	                                                i - 1
	                                            }
	                                        }
	                                        None => 0,
	                                    };
	                                    app.lsp_state.select(Some(i));
	                                } else if app.tab_index == 6 { // Settings
	                                    app.settings_option = match app.settings_option {
	                                        SettingsOption::Editor => SettingsOption::Save,
	                                        SettingsOption::Theme => SettingsOption::Editor,
	                                        SettingsOption::InstallPath => SettingsOption::Theme,
	                                        SettingsOption::Save => SettingsOption::InstallPath,
	                                    };
	                                }
                                app.scroll = app.scroll.saturating_sub(1);
                            }
                            (_, KeyCode::Char('1')) => app.tab_index = 0,
                            (_, KeyCode::Char('2')) => app.tab_index = 1,
                            (_, KeyCode::Char('3')) => app.tab_index = 2,
                            (_, KeyCode::Char('4')) => app.tab_index = 3,
                            (_, KeyCode::Char('5')) => app.tab_index = 4,
                            (_, KeyCode::Char('6')) => app.tab_index = 5,
                            (_, KeyCode::Char('7')) => app.tab_index = 6,
                            (_, KeyCode::Char('G')) => {
                                if app.tab_index == 1 {
                                    app.input_mode = InputMode::NewGroupTitle;
                                    app.rename_buffer.clear();
                                    app.new_group_category.clear();
                                }
                            }
                            (_, KeyCode::Char('m')) => {
                                if app.tab_index == 1 {
                                    if let Some(i) = app.session_state.selected() {
                                        if let Some(SessionEntry::Session(s)) = app.session_entries.get(i) {
                                            app.target_session_id = Some(s.session_id.clone());
                                            app.input_mode = InputMode::MoveToGroup;
                                            app.rename_buffer.clear();
                                        }
                                    }
                                }
                            }
                            (_, KeyCode::Char('a')) => {
                                if app.tab_index == 3 { // Analysis tab
                                    app.input_mode = InputMode::AnalysisPrompt;
                                }
                            }
                            (_, KeyCode::Right) => {
                                if app.tab_index == 2 && app.preview_focused {
                                    if let (Some(_), Some(path)) = (app.project_state.selected(), &app.project_explorer_path) {
                                        if let Some(item_name) = app.explorer_items.get(app.project_explorer_selected) {
                                            let mut new_path = std::path::PathBuf::from(path.replace("~", &std::env::var("HOME").unwrap_or_default()));
                                            new_path.push(item_name);
                                            if new_path.is_dir() {
                                                app.project_explorer_path = Some(new_path.to_string_lossy().to_string());
                                                app.project_explorer_selected = 0;
                                            } else {
                                                let _ = terminal.clear();
                                                let editor = &app.config.editor;
                                                let _ = std::process::Command::new(editor).arg(new_path).status();
                                                let _ = terminal.clear();
                                            }
                                        }
                                    }
                                } else if app.tab_index == 1 {
                                    if let Some(i) = app.session_state.selected() {
                                        if let Some(SessionEntry::Group(g)) = app.session_entries.get(i) {
                                            if !g.is_expanded {
                                                if let Some(group) = app.groups.iter_mut().find(|group| group.path == g.path) {
                                                    group.is_expanded = true;
                                                    if let Some(svc) = service.as_ref() { let _ = svc.update_group_expansion(&group.path, true); }
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
                                            app.project_explorer_path = Some(parent.to_string_lossy().to_string());
                                            app.project_explorer_selected = 0;
                                        }
                                    }
                                } else if app.tab_index == 1 {
                                    if let Some(i) = app.session_state.selected() {
                                        if let Some(SessionEntry::Group(g)) = app.session_entries.get(i) {
                                            if g.is_expanded {
                                                if let Some(group) = app.groups.iter_mut().find(|group| group.path == g.path) {
                                                    group.is_expanded = false;
                                                    if let Some(svc) = service.as_ref() { let _ = svc.update_group_expansion(&group.path, false); }
                                                    app.refresh_session_entries();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            (_, KeyCode::Backspace) if app.tab_index == 2 && app.preview_focused => {
                                if let Some(current) = &app.project_explorer_path {
                                    let path = std::path::PathBuf::from(current.replace("~", &std::env::var("HOME").unwrap_or_default()));
                                    if let Some(parent) = path.parent() {
                                        app.project_explorer_path = Some(parent.to_string_lossy().to_string());
                                        app.project_explorer_selected = 0;
                                    }
                                }
                            }
	                            (_, KeyCode::Char('/') | KeyCode::Char('?')) => {
	                                app.show_help = true;
	                                app.help_scroll = 0;
	                            }
                            (_, KeyCode::Char('e')) if app.tab_index == 2 => {
                                app.preview_focused = !app.preview_focused;
                            }
                              (_, KeyCode::Enter) => {
                                  if app.tab_index == 0 { // Dashboard
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
                                                      Ok(()) => format!("Returned from '{}'", s.title),
                                                      Err(e) => format!("Attach failed: {}", e),
                                                  };
                                              }
                                          }
                                          DashFocus::Mcp => {
                                              if let Some(i) = app.mcp_state.selected() {
                                                  if let Some(mcp) = app.mcp_servers.get(i) {
                                                     app.target_mcp_name = Some(mcp.name.clone());
                                                     app.input_mode = InputMode::McpMenu;
                                                     app.mcp_menu_option = McpOption::StartStop;
                                                 }
                                             }
                                         }
                                         DashFocus::Tabs => {}
                                     }
	                                 } else if app.tab_index == 6 { // Settings
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
	                                        SettingsOption::Save => {
	                                            let _ = app.config.save();
	                                            app.status_message = "Configuration saved to ~/.config/maestro/config.toml".to_string();
	                                        }
	                                    }
	                                } else if app.tab_index == 2 { // Projects Tab
                                    if let Some(i) = app.project_state.selected() {
                                        let project = &app.projects[i].clone();
                                        app.status_message = format!("Launching Zide for {}...", project.name);
                                        let _ = terminal.draw(|frame| ui(frame, &mut app));

                                        let _ = suspend_fullscreen_app(terminal);
                                        let res = leindex_analyzers::multiplexer::zellij::ZellijMultiplexer::spawn_zide(&project.path, &project.name);
                                        let _ = resume_fullscreen_app(terminal);

                                        match res {
                                            Ok(_) => {
                                                let _ = terminal.clear(); // Ensure screen is clear after Zellij exit
                                                let _ = terminal.draw(|frame| ui(frame, &mut app));
                                                app.status_message = format!("Returned from Zide for {}.", project.name);
                                            }
                                            Err(e) => {
                                                app.status_message = format!("Error: {}", e);
                                            }
                                        }
                                    }
                                } else if app.tab_index == 1 { // Sessions Tab
                                    if let Some(i) = app.session_state.selected() {
                                        if let Some(entry) = app.session_entries.get(i).cloned() {
                                            match entry {
                                                SessionEntry::Group(g) => {
                                                    if let Some(group) = app.groups.iter_mut().find(|group| group.path == g.path) {
                                                        group.is_expanded = !group.is_expanded;
                                                        if let Some(svc) = service.as_ref() {
                                                            let _ = svc.update_group_expansion(&group.path, group.is_expanded);
                                                        }
                                                        app.refresh_session_entries();
                                                    }
                                                }
	                                                SessionEntry::Session(s) => {
	                                                    app.status_message = format!("Attaching to '{}'... (Ctrl+B d to detach)", s.title);
	                                                    let _ = terminal.draw(|frame| ui(frame, &mut app));
	                                                    let _ = suspend_fullscreen_app(terminal);
	                                                    let res = TmuxMultiplexer::attach(&s.session_id);
	                                                    let _ = resume_fullscreen_app(terminal);
	                                                    let _ = terminal.clear(); // Restore terminal state
	                                                    match res {
	                                                        Ok(()) => {
	                                                            app.status_message =
	                                                                format!("Returned from '{}'", s.title);
	                                                        }
	                                                        Err(e) => {
	                                                            // If the session is dead, do the useful thing:
	                                                            // recreate the shell and attempt to resume the agent (best-effort).
	                                                            if s.status
	                                                                == leindex_analyzers::memory::models::SessionStatus::Terminated
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

	                                                                    if let Ok(manager) = leindex_analyzers::memory::session_manager::SessionManager::new(
	                                                                        svc.clone(),
	                                                                    ) {
	                                                                        let _ = manager.restore_session(
	                                                                            &s,
	                                                                            leindex_analyzers::memory::session_manager::SessionRestoreMode::Resume,
	                                                                        );
	                                                                    }
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


                            (_, KeyCode::Char('s')) => {
                                if app.tab_index == 0 { // Dashboard (MCP)
                                    if let Some(i) = app.mcp_state.selected() {
                                        if let Some(mcp) = app.mcp_servers.get(i) {
                                            app.target_mcp_name = Some(mcp.name.clone());
                                            app.input_mode = InputMode::McpMenu;
                                            app.mcp_menu_option = McpOption::StartStop;
                                        }
                                    }
                                } else if app.tab_index == 5 { // LSPs tab
                                    // Toggle LSP start/stop
                                    if let Some((session_id, lsp_name, status)) = app.get_selected_lsp() {
                                        app.toggle_lsp(&session_id, &lsp_name, status);
                                    } else {
                                        app.status_message = "No LSP selected".to_string();
                                    }
                                } else {
                                    app.input_mode = InputMode::SessionSwitcher;
                                    app.switcher_state.select(Some(0));
                                }
                            }
                            (_, KeyCode::Char('x')) => {
                                if app.tab_index == 0 { // Remove MCP
                                    if let Some(i) = app.mcp_state.selected() {
                                        let name = app.mcp_servers[i].name.clone();
                                        if let Some(svc) = service.as_ref() {
                                            let _ = svc.delete_mcp_server(&name);
                                            if let Ok(mcp_list) = svc.list_mcp_servers() { app.mcp_servers = mcp_list; }
                                        }
                                    }
                                }
                            }
                            (_, KeyCode::Char('n')) => {
                                app.input_mode = InputMode::NewSessionTitle;
                                // Auto-fill path if a project is selected
                                if app.tab_index == 2 { // Projects Tab
                                    if let Some(i) = app.project_state.selected() {
                                        app.new_session_path = app.projects[i].path.clone();
                                        app.new_session_title = format!("Chat: {}", app.projects[i].name);
                                    }
                                }
                            }
	                            (_, KeyCode::Char('R')) => {
	                                // Sessions tab: restart the selected session (shell/tool fresh).
	                                if app.tab_index == 1 {
	                                    let Some(i) = app.session_state.selected() else { continue; };
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

	                                    let manager = match leindex_analyzers::memory::session_manager::SessionManager::new(
	                                        svc.clone(),
	                                    ) {
	                                        Ok(m) => m,
	                                        Err(e) => {
	                                            app.status_message =
	                                                format!("Failed to create session manager: {}", e);
	                                            app.is_spawning = false;
	                                            continue;
	                                        }
	                                    };

	                                    let res = manager.restore_session(
	                                        &s,
	                                        leindex_analyzers::memory::session_manager::SessionRestoreMode::Restart,
	                                    );
	                                    app.is_spawning = false;
	                                    app.refresh_from_service(&service);

	                                    match res {
	                                        Ok(()) => {
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
	                                        Err(e) => {
	                                            app.status_message = format!("Restart failed: {}", e);
	                                        }
	                                    }
	                                } else if app.tab_index == 5 { // LSPs tab
	                                    // Restart LSP
	                                    if let Some((session_id, lsp_name, _status)) = app.get_selected_lsp() {
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
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Content
            Constraint::Length(3),  // Footer
        ])
        .split(frame.area());

    // Header with tabs
    let is_focused = app.tab_index == 0 && app.dash_focus == DashFocus::Tabs;
    let tabs = Tabs::new(vec!["Dashboard", "Sessions", "Projects", "Analysis", "Memory", "LSPs", "Settings"])
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(if is_focused { BorderType::Double } else { BorderType::Rounded })
            .border_style(if is_focused { Style::default().fg(theme.warning).bold() } else { Style::default().fg(theme.muted) })
            .title(" Maestro Cockpit v2.0 "))
        .select(app.tab_index)
        .highlight_style(Style::default().fg(theme.accent).bold());

    frame.render_widget(tabs, chunks[0]);

    match app.tab_index {
        0 => render_dashboard(frame, chunks[1], app),
        1 => render_sessions(frame, chunks[1], app),
        2 => render_projects(frame, chunks[1], app),
        3 => render_analysis(frame, chunks[1], app),
        4 => render_memory(frame, chunks[1], app),
        5 => render_lsps(frame, chunks[1], app),
        6 => render_settings(frame, app),
        _ => {}
    }
    // Footer
    let footer = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" Tab ", Style::default().bg(Color::Cyan).fg(Color::Black)),
            Span::raw(" Switch  "),
            Span::styled(" ↑↓ Arrows ", Style::default().bg(Color::Cyan).fg(Color::Black)),
            Span::raw(" Scroll  "),
            Span::styled(" 1-7 ", Style::default().bg(Color::Cyan).fg(Color::Black)),
            Span::raw(" Jump  "),
            Span::styled(" n ", Style::default().bg(Color::Green).fg(Color::Black)),
            Span::raw(" New  "),
            Span::styled(" s ", Style::default().bg(Color::Magenta).fg(Color::Black)),
            Span::raw(" Switch "),
            Span::styled(" / ", Style::default().bg(Color::Yellow).fg(Color::Black)),
            Span::raw(" Help "),
            if std::env::var("ZELLIJ").is_ok() {
                Span::styled(" [Zellij Active: Ctrl+G for menu] ", Style::default().fg(Color::Yellow).bold())
            } else {
                Span::raw("")
            },
            Span::styled(" q ", Style::default().bg(Color::Red).fg(Color::White)),
            Span::raw(" Quit"),
        ])
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
    );
    frame.render_widget(footer, chunks[2]);

    // Render Modals
    if app.show_help {
        render_help_modal(frame, app);
    }
    
    // Only show these modals if they overlay the main tabs appropriately
	    if app.input_mode == InputMode::SessionSwitcher {
	        render_switcher_modal(frame, app);
	    } else if app.input_mode == InputMode::SessionHub {
	        render_session_hub_modal(frame, app);
	    } else if app.input_mode == InputMode::McpMenu {
	        render_mcp_menu(frame, app);
	    } else if app.input_mode == InputMode::McpLogs {
	        render_mcp_logs_modal(frame, app);
	    } else if app.input_mode == InputMode::SettingsMenu {
	        render_settings_menu_modal(frame, app);
	    } else if matches!(app.input_mode, InputMode::NewProjectName | InputMode::NewProjectPath | InputMode::NewProjectTool) {
	        render_new_project_modal(frame, app);
	    } else if matches!(app.input_mode, InputMode::NewTrackTitle | InputMode::NewTrackType) {
	        render_new_track_modal(frame, app);
    } else if matches!(app.input_mode, InputMode::NewGroupTitle | InputMode::NewGroupCategory | InputMode::RenameGroup | InputMode::RenameGroupCategory) {
        render_group_modal(frame, app);
    } else if matches!(app.input_mode, InputMode::ForkSession | InputMode::KillConfirm | InputMode::DeleteConfirm | InputMode::MoveToGroup) {
        render_action_modal(frame, app);
    } else if matches!(app.input_mode, InputMode::NewSessionTitle | InputMode::NewSessionPath | InputMode::NewSessionTool) {
        render_input_modal(frame, app);
    }

    if app.is_spawning {
        render_spawning_overlay(frame, app);
    }
}

fn render_action_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, frame.area());
    frame.render_widget(Clear, area);

    let (title, prompt, value) = match app.input_mode {
        InputMode::RenameGroup => (" Rename Group ", "New Name:", Some(&app.rename_buffer)),
        InputMode::ForkSession => (" Fork Session ", "Fork Name:", Some(&app.rename_buffer)),
        InputMode::KillConfirm => (" Kill Session ", "Are you sure? (y/n)", None),
        InputMode::DeleteConfirm => (" Permanent Delete ", "Are you sure you want to PERMANENTLY delete? (y/n)", None),
        InputMode::NewSessionTitle => (" New Session ", "Enter Title:", Some(&app.new_session_title)),
        InputMode::NewGroupTitle => (" New Group ", "Group Name:", Some(&app.rename_buffer)),
        InputMode::MoveToGroup => (" Move to Group ", "Target Path:", Some(&app.rename_buffer)),
        _ => ("", "", None),
    };

    let theme = app.theme();
    let title_style = match app.input_mode {
        InputMode::KillConfirm | InputMode::DeleteConfirm => Style::default().fg(theme.error).bold(),
        _ => Style::default().fg(theme.warning).bold(),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(title)
        .title_style(title_style);

    let content = if let Some(v) = value {
        format!("{}\n\n> {}", prompt, v)
    } else {
        prompt.to_string()
    };

    let para = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}

fn render_spawning_overlay(frame: &mut Frame, app: &App) {
    let area = centered_rect(40, 10, frame.area());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(30, 0, 30)).fg(Color::Yellow));

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚡ ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&app.status_message),
        ]),
    ];

    let para = Paragraph::new(text).block(block).alignment(Alignment::Center);
    frame.render_widget(Clear, area);
    frame.render_widget(para, area);
}

fn render_dashboard(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(chunks[0]);

    // Stats cards
    let stats_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" ⚡ Quick Stats ")
        .title_style(Style::default().fg(Color::Yellow));
    
    let stats_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  📁 PROJECTS:   ", Style::default().fg(Color::Cyan)),
            Span::styled(format!("{:02}", app.stats.project_count), Style::default().fg(Color::Green).bold()),
            Span::styled("  [Active System Roots]", Style::default().fg(Color::DarkGray).italic()),
        ]),
        Line::from(vec![
            Span::styled("  🎯 TRACKS:     ", Style::default().fg(Color::Cyan)),
            Span::styled(format!("{:02}", app.stats.track_count), Style::default().fg(Color::Green).bold()),
            Span::styled("  [Active Workstreams]", Style::default().fg(Color::DarkGray).italic()),
        ]),
        Line::from(vec![
            Span::styled("  🧠 MEMORIES:   ", Style::default().fg(Color::Cyan)),
            Span::styled(format!("{:02}", app.stats.memory_count), Style::default().fg(Color::Green).bold()),
            Span::styled("  [Context Vectors]", Style::default().fg(Color::DarkGray).italic()),
        ]),
        Line::from(vec![
            Span::styled("  ⚡ LEINDEX:    ", Style::default().fg(Color::Cyan)),
            Span::styled("HD", Style::default().fg(Color::Yellow).bold()),
            Span::styled("  [Multi-Layer structural cache]", Style::default().fg(Color::DarkGray).italic()),
        ]),
    ];
    let stats = Paragraph::new(stats_text).block(stats_block);
    frame.render_widget(stats, left_chunks[0]);

    // Welcome message
    let welcome_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Welcome ")
        .title_style(Style::default().fg(Color::Magenta));
    
    // Updated welcome section with multi-layer architecture diagram & ANIMATION
    let anim_char = match (app.frame_count / 10) % 4 {
        0 => "⠋", 1 => "⠙", 2 => "⠹", _ => "⠸",
    };
    let welcome_color = if (app.frame_count / 20) % 2 == 0 { Color::Magenta } else { Color::LightMagenta };

    let welcome_text = vec![
        Line::from(vec![
            Span::styled(format!(" {} MAESTRO SYSTEM OVERVIEW ", anim_char), Style::default().fg(welcome_color).bold()),
            Span::styled(" [v2.0-beta-5]", Style::default().fg(Color::DarkGray).italic()),
        ]),
        Line::from(""),
        Line::from("  [WORKSPACE] ─────▶ [SCANNER] ─────▶ [LEINDEXER]"),
        Line::from("       │                │                │"),
        Line::from("       ▼                ▼                ▼"),
        Line::from("  [CONFIGS]        [TRACKS]         [MEMORY DB]"),
        Line::from("       │                │                │"),
        Line::from("       └──────┬─────────┴────────────────┘"),
        Line::from("              ▼"),
        Line::from(vec![
            Span::styled("      [ AI AGENT LAYER ]", Style::default().fg(Color::LightMagenta).bold().add_modifier(Modifier::DIM)),
        ]),
        Line::from("      (Claude / Gemini / Codex / AMP)"),
        Line::from(""),
        Line::from(vec![
            Span::styled("  🚀 CAPABILITIES & FEATURES:", Style::default().bold().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("    ● ", Style::default().fg(Color::Green)),
            Span::styled("Indexing: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("N-Layer vector search via LEANN."),
            Span::styled(" (Example: 'scan /path/to/repo')", Style::default().fg(Color::DarkGray).italic()),
        ]),
        Line::from(vec![
            Span::styled("    ● ", Style::default().fg(Color::Cyan)),
            Span::styled("Sessions: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Persistent tmux environments."),
            Span::styled(" (Example: 'n' to spawn)", Style::default().fg(Color::DarkGray).italic()),
        ]),
        Line::from(vec![
            Span::styled("    ● ", Style::default().fg(Color::Magenta)),
            Span::styled("Analysis: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Structural code intelligence."),
            Span::styled(" (Example: 'analyze src/')", Style::default().fg(Color::DarkGray).italic()),
        ]),
        Line::from(vec![
            Span::styled("    ● ", Style::default().fg(Color::Blue)),
            Span::styled("Memory:   ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Global cross-project knowledge."),
            Span::styled(" (Example: Tab 5)", Style::default().fg(Color::DarkGray).italic()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Maestro is your autonomous coding cockpit. ", Style::default().fg(Color::LightBlue).italic()),
            Span::styled("Stay playful, build fast!", Style::default().fg(Color::Yellow).bold()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("'/'", Style::default().fg(Color::Yellow).bold()),
            Span::styled(" for the Ultimate Command Guide", Style::default().fg(Color::DarkGray)),
        ]),
    ];
    let welcome = Paragraph::new(welcome_text).block(welcome_block);
    frame.render_widget(welcome, left_chunks[1]);

    // Right side - System Status & MCP Pool
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(10)])
        .split(chunks[1]);

    // Top Right - Recent Sessions
    let session_block = Block::default()
        .borders(Borders::ALL)
        .border_type(if app.tab_index == 0 && app.dash_focus == DashFocus::Sessions { BorderType::Double } else { BorderType::Rounded })
        .title(" 🕒 Recent Sessions ")
        .title_style(if app.tab_index == 0 && app.dash_focus == DashFocus::Sessions { Style::default().fg(Color::Blue).bold() } else { Style::default().fg(Color::Blue) });

    let mut session_items = Vec::new();
    if app.dash_session_entries.is_empty() {
        session_items.push(ListItem::new("  No active sessions"));
    } else {
        // Pre-collect LSP indicators for all sessions (single borrow, then release)
        // Clone the cache first to avoid borrow checker issues
        let lsp_cache = app.lsp_status_cache.clone();
        let lsp_indicators_map = {
            let mut map = std::collections::HashMap::new();
            for (session_id, lsps) in &lsp_cache {
                if !lsps.is_empty() {
                    let indicators: Vec<Span> = lsps
                        .iter()
                        .map(|(lsp_name, status)| {
                            let (icon, color) = match status {
                                LspStatus::Running => (" ● ", Color::Green),
                                LspStatus::Starting => (" ◐ ", Color::Yellow),
                                LspStatus::Error => (" x ", Color::Red),
                                LspStatus::Stopped => (" ○ ", Color::Gray),
                            };
                            let short_name = if lsp_name.contains("rust") {
                                "R"
                            } else if lsp_name.contains("ruff") || lsp_name.contains("python") {
                                "P"
                            } else if lsp_name.contains("typescript") || lsp_name.contains("ts") {
                                "T"
                            } else {
                                "?"
                            };
                            Span::styled(
                                format!("{}{}", short_name, icon),
                                Style::default().fg(color),
                            )
                        })
                        .collect();
                    map.insert(session_id.clone(), indicators);
                }
            }
            map
        };
        // `app` borrow is now released

        for entry in &app.dash_session_entries {
            match entry {
                DashSessionEntry::GroupHeader { group_path } => {
                    let group_name = if group_path == "uncategorized" {
                        "[Uncategorized]".to_string()
                    } else {
                        app.groups
                            .iter()
                            .find(|g| g.path == *group_path)
                            .map(|g| g.name.clone())
                            .unwrap_or_else(|| group_path.to_string())
                    };
                    session_items.push(ListItem::new(Line::from(vec![Span::styled(
                        format!(" 📁 {} ", group_name),
                        Style::default().fg(Color::Yellow).bold(),
                    )])));
                }
                DashSessionEntry::Session(sess) => {
                    let status_icon = match sess.status {
                        leindex_analyzers::memory::models::SessionStatus::Running => {
                            Span::styled(" ● ", Style::default().fg(Color::Green))
                        }
                        leindex_analyzers::memory::models::SessionStatus::Terminated => {
                            Span::styled(" x ", Style::default().fg(Color::Red))
                        }
                        leindex_analyzers::memory::models::SessionStatus::Waiting => {
                            Span::styled(" ◒ ", Style::default().fg(Color::Yellow))
                        }
                        _ => Span::styled(" o ", Style::default().fg(Color::Gray)),
                    };

                    // Build line with session status, title, and LSP indicators
                    let mut line_spans = vec![
                        Span::raw("   "),
                        status_icon,
                        Span::styled(sess.title.clone(), Style::default().bold()),
                    ];

                    // Add LSP indicators if any exist (use pre-collected map)
                    if let Some(indicators) = lsp_indicators_map.get(&sess.session_id) {
                        if !indicators.is_empty() {
                            line_spans.push(Span::raw(" "));
                            line_spans.push(Span::styled("LSP:", Style::default().fg(Color::DarkGray)));
                            for indicator in indicators {
                                line_spans.push(indicator.clone());
                            }
                        }
                    }

                    session_items.push(ListItem::new(Line::from(line_spans)));
                }
            }
        }
    }
    let sessions = List::new(session_items)
        .block(session_block)
        .highlight_style(Style::default().bg(theme.highlight_bg).fg(theme.highlight_fg).bold())
        .highlight_symbol(">> ");
    frame.render_stateful_widget(sessions, right_chunks[0], &mut app.dash_session_state);

	    // Bottom Right - MCP Pool
		    let mcp_block = Block::default()
		        .borders(Borders::ALL)
		        .border_type(if app.tab_index == 0 && app.dash_focus == DashFocus::Mcp { BorderType::Double } else { BorderType::Rounded })
		        .title(" 🕹️ Interactive MCP Pool ")
		        .title_style(if app.tab_index == 0 && app.dash_focus == DashFocus::Mcp { Style::default().fg(theme.accent).bold() } else { Style::default().fg(theme.accent) });

	    let mcp_chunks = Layout::default()
	        .direction(Direction::Vertical)
	        .constraints([Constraint::Length(2), Constraint::Min(0)])
	        .split(right_chunks[1]);

		    let mcp_info = Paragraph::new(vec![
		        Line::from(vec![
		            Span::styled("Tip: ", Style::default().fg(theme.muted)),
		            Span::styled("Tool Search", Style::default().fg(theme.warning).bold()),
		            Span::styled(
		                " is dynamic via `maestro mcp tool-search` (no full tool listing).",
		                Style::default().fg(theme.muted),
		            ),
		        ]),
		    ])
	    .block(Block::default())
	    .wrap(Wrap { trim: true });
	    frame.render_widget(mcp_info, mcp_chunks[0]);

	    let mcp_items: Vec<ListItem> = app.mcp_servers.iter().map(|s| {
	        let status_color = if s.status == leindex_analyzers::memory::models::McpStatus::Running { Color::Green } else { Color::Red };
	        ListItem::new(vec![
	            Line::from(vec![
	                Span::styled(format!("  {} ", s.name), Style::default().bold()),
                Span::styled(format!(" [{}] ", s.status.to_string()), 
                    Style::default().fg(status_color)),
                Span::styled(format!(" {} active", s.client_count), Style::default().fg(Color::Gray)),
            ]),
        ])
    }).collect();

		    let mcp_list = List::new(mcp_items)
		        .block(mcp_block)
		        .highlight_style(Style::default().bg(theme.highlight_bg).fg(theme.highlight_fg).bold())
		        .highlight_symbol(">> ");
		    frame.render_stateful_widget(mcp_list, mcp_chunks[1], &mut app.mcp_state);
}

fn render_help_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 40, frame.area());
    let theme = theme_from_name(&app.config.theme);
    let block = Block::default()
        .title(" Commands Cheat-sheet ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(theme.panel_bg));

    let text = build_help_text(app);

    let para = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Left)
        .scroll((app.help_scroll, 0))
        .wrap(Wrap { trim: true });
    frame.render_widget(Clear, area);
    frame.render_widget(para, area);
}

fn build_help_text(app: &App) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(" GLOBAL CONTROLS:", Style::default().fg(Color::Yellow).bold())]),
        Line::from(vec![Span::styled("   Tab / S-Tab   ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Cycle Tabs / Focus Preview (e.g. 1->2->3)")]),
        Line::from(vec![Span::styled("   ↑ / ↓         ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Navigate / Scroll Preview")]),
        Line::from(vec![Span::styled("   / or ?        ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Open/close this modal")]),
        Line::from(vec![Span::styled("   PgUp/PgDn     ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Scroll modal content")]),
        Line::from(vec![Span::styled("   q / Ctrl-C    ", Style::default().fg(Color::Red).bold()),  Span::raw(" Quit Maestro Cockpit")]),
        Line::from(""),
        Line::from(vec![Span::styled("   Dash: k / d   ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Kill / Delete Highlighted Dashboard Session")]),
        Line::from(""),
        Line::from(vec![Span::styled(" SESSIONS (Tab 2):", Style::default().fg(Color::Yellow).bold())]),
        Line::from(vec![Span::styled("   n             ", Style::default().fg(Color::Green).bold()), Span::raw(" New Session Wizard (Title, Path, Tool)")]),
        Line::from(vec![Span::styled("   Enter         ", Style::default().fg(Color::Green).bold()), Span::raw(" Attach (auto-resume if terminated)")]),
        Line::from(vec![Span::styled("   u             ", Style::default().fg(Color::Green).bold()), Span::raw(" Resume (restore shell + resume agent, best-effort)")]),
        Line::from(vec![Span::styled("   R             ", Style::default().fg(Color::Green).bold()), Span::raw(" Restart (restore shell + start tool fresh)")]),
        Line::from(vec![Span::styled("   r             ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Session Hub (Rename, Move, Search history)")]),
        Line::from(vec![Span::styled("   Alt + p       ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Focus Preview Pane (for scrolling history)")]),
        Line::from(vec![Span::styled("   Alt + ↑/↓     ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Reorder group/session (persists to DB)")]),
        Line::from(vec![Span::styled("   m             ", Style::default().fg(Color::Magenta).bold()), Span::raw(" Move Session to Group / Create New Group")]),
        Line::from(vec![Span::styled("   G             ", Style::default().fg(Color::Green).bold()), Span::raw(" Create Standalone Group")]),
        Line::from(vec![Span::styled("   k             ", Style::default().fg(Color::Red).bold()), Span::raw(" Kill tmux Session Process")]),
        Line::from(vec![Span::styled("   d / Alt + D   ", Style::default().fg(Color::Red).bold()), Span::raw(" PURMANENT DELETE Session/Group from DB")]),
        Line::from(vec![Span::styled("   f             ", Style::default().fg(Color::Magenta).bold()), Span::raw(" Fork Session (Clone state to new session)")]),
        Line::from(""),
        Line::from(vec![Span::styled(" MEMORY (Tab 5):", Style::default().fg(Color::Yellow).bold())]),
        Line::from(vec![Span::styled("   Ctrl + f      ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Search memories (hybrid Tantivy/SQLite)")]),
        Line::from(vec![Span::styled("   Ctrl + l      ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Clear memory search")]),
        Line::from(vec![Span::styled("   r             ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Refresh/import system-wide memories")]),
        Line::from(""),
        Line::from(vec![Span::styled(" PROJECTS (Tab 3):", Style::default().fg(Color::Yellow).bold())]),
        Line::from(vec![Span::styled("   Enter         ", Style::default().fg(Color::Green).bold()), Span::raw(" Open Zide (File Picker + Editor)")]),
        Line::from(""),
        Line::from(vec![Span::styled(" ANALYSIS (Tab 4):", Style::default().fg(Color::Yellow).bold())]),
        Line::from(vec![Span::styled("   a             ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Enter Analysis Command Box")]),
        Line::from(""),
        Line::from("  ---------------------------------- "),
        Line::from(format!("  Maestro TUI Cockpit v2.0-beta-8  {}", if (app.frame_count / 30) % 2 == 0 { "⚡" } else { "  " })),
    ]
}

fn render_session_hub_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(80, 60, frame.area());
    frame.render_widget(Clear, area);
    
    let block = Block::default()
        .title(" SESSION HUB Control ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(10, 10, 15)));
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // RENAME
            Constraint::Length(3), // GROUP
            Constraint::Min(0),    // PREVIEW
            Constraint::Length(3), // SEARCH
        ])
        .split(area);

    // Rename Box
    let rename_style = if app.hub_focus == HubFocus::Rename { Style::default().fg(Color::Yellow).bold() } else { Style::default() };
    let rename_title = if app.hub_focus == HubFocus::Rename { ">> RENAME (Enter to Commit) " } else { " RENAME " };
    let rename = Paragraph::new(app.rename_buffer.as_str())
        .block(Block::default().borders(Borders::ALL).title(rename_title).border_style(rename_style));
    frame.render_widget(rename, chunks[0]);

    // Group Box
    let group_style = if app.hub_focus == HubFocus::Group { Style::default().fg(Color::Cyan).bold() } else { Style::default() };
    let group_title = if app.hub_focus == HubFocus::Group { ">> GROUP ASSIGNMENT (Enter to change) " } else { " GROUP ASSIGNMENT " };
    let group = Paragraph::new("Current: /default (Press 'm' to Move)")
        .block(Block::default().borders(Borders::ALL).title(group_title).border_style(group_style));
    frame.render_widget(group, chunks[1]);
    
    // Search Results / Pane Preview
    let preview = Paragraph::new(app.session_preview_content.as_str())
        .block(Block::default().borders(Borders::ALL).title(" PANE HISTORY PREVIEW / SEARCH RESULTS "))
        .wrap(Wrap { trim: false });
    frame.render_widget(preview, chunks[2]);

    // Search Input
    let search_style = if app.hub_focus == HubFocus::Search { Style::default().fg(Color::Magenta).bold() } else { Style::default() };
    let search_title = if app.hub_focus == HubFocus::Search { ">> SEARCH IN PANE (Type to filter) " } else { " SEARCH IN PANE " };
    let search_content = if app.hub_focus == HubFocus::Search { format!("{}_", app.hub_search_buffer) } else { app.hub_search_buffer.clone() };
    let search_input = Paragraph::new(search_content)
        .block(Block::default().borders(Borders::ALL).title(search_title).border_style(search_style));
    frame.render_widget(search_input, chunks[3]);
}

fn render_mcp_menu(frame: &mut Frame, app: &App) {
    let area = centered_rect(40, 40, frame.area());
    frame.render_widget(Clear, area);
    let theme = theme_from_name(&app.config.theme);
    
    let name = app.target_mcp_name.as_deref().unwrap_or("Unknown");
    let block = Block::default()
        .title(format!(" MCP: {} ", name))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(theme.panel_bg));
    
    let options = vec![
        (McpOption::StartStop, "▶/■ Start/Stop Server"),
        (McpOption::Pause, "⏸ Pause Connection"),
        (McpOption::Logs, "📋 View Server Logs"),
        (McpOption::Add, "➕ Add New Server"),
        (McpOption::Remove, "❌ Remove from Pool"),
        (McpOption::Install, "🛠️ Install Component"),
    ];

    let mut list_items = Vec::new();
    for (opt, label) in options {
        let style = if app.mcp_menu_option == opt {
            Style::default().fg(Color::Yellow).bold().bg(Color::Rgb(40, 40, 60))
        } else {
            Style::default()
        };
        list_items.push(ListItem::new(vec![Line::from(vec![
            Span::styled(if app.mcp_menu_option == opt { " >> " } else { "    " }, style),
            Span::styled(label, style),
        ])]));
    }

    let list = List::new(list_items).block(block);
    frame.render_widget(list, area);
}

fn render_mcp_logs_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(80, 70, frame.area());
    frame.render_widget(Clear, area);

    // Determine if we're showing MCP or LSP logs
    let is_lsp_logs = app.lsp_log_source.is_some();

    let (title, content, scroll_offset) = if is_lsp_logs {
        // LSP logs
        let (session_id, lsp_name) = app.lsp_log_source.as_ref().unwrap();
        let title = format!(" LSP Logs: {} - Session {} (Esc to close) ", lsp_name, session_id);
        let content = if app.lsp_log_content.is_empty() {
            vec![
                Line::from(""),
                Line::from("  No logs found."),
                Line::from(""),
                Line::from("  Tip: LSP logs may not be enabled for this server."),
            ]
        } else {
            app.lsp_log_content.lines().map(|l| Line::from(l)).collect()
        };
        let scroll_offset = (app.lsp_log_scroll, 0);
        (title, content, scroll_offset)
    } else {
        // MCP logs
        let name = app.target_mcp_name.as_deref().unwrap_or("Unknown");
        let title = format!(" MCP Logs: {} (Esc to close) ", name);
        let content = if app.mcp_log_lines.is_empty() {
            vec![
                Line::from(""),
                Line::from("  No logs found."),
                Line::from(""),
                Line::from("  Tip: start the server to generate logs."),
            ]
        } else {
            app.mcp_log_lines.iter().map(|l| Line::from(l.as_str())).collect()
        };
        let scroll_offset = (app.mcp_log_scroll, 0);
        (title, content, scroll_offset)
    };

    let theme = theme_from_name(&app.config.theme);
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(theme.panel_bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let para = Paragraph::new(content)
        .scroll(scroll_offset)
        .wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}

fn render_settings_menu_modal(frame: &mut Frame, app: &mut App) {
    let theme = theme_from_name(&app.config.theme);
    let area = centered_rect(60, 60, frame.area());
    frame.render_widget(Clear, area);

    let title = match app.settings_menu_kind {
        Some(SettingsMenuKind::Editor) => " Select Preferred Editor ",
        Some(SettingsMenuKind::Theme) => " Select Theme ",
        None => " Select ",
    };

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(theme.panel_bg));

    let items: Vec<ListItem> = app
        .settings_menu_items
        .iter()
        .map(|(_, label)| ListItem::new(label.clone()))
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg)
                .bold(),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, &mut app.settings_menu_state);
}

fn render_settings(frame: &mut Frame, app: &App) {
    let theme = theme_from_name(&app.config.theme);
    let area = frame.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ⚙️ SYSTEM SETTINGS ")
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent));
    
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Editor
            Constraint::Length(3), // Theme
            Constraint::Length(3), // Install Path
            Constraint::Length(3), // Save button
            Constraint::Min(0),
        ])
        .split(inner_area);

    let editor_style = if app.tab_index == 6 && app.settings_option == SettingsOption::Editor { Style::default().fg(theme.warning).bold() } else { Style::default() };
    let editor = Paragraph::new(app.config.editor.as_str())
        .block(Block::default().borders(Borders::ALL).title(" 📝 PREFERRED EDITOR ").border_style(editor_style));
    frame.render_widget(editor, chunks[0]);

    let theme_style = if app.tab_index == 6 && app.settings_option == SettingsOption::Theme { Style::default().fg(theme.warning).bold() } else { Style::default() };
    let theme_name = THEMES
        .iter()
        .find(|(id, _)| id.eq_ignore_ascii_case(app.config.theme.as_str()))
        .map(|(_, label)| *label)
        .unwrap_or("Custom");
    let theme_field = Paragraph::new(theme_name)
        .block(Block::default().borders(Borders::ALL).title(" 🎨 THEME ").border_style(theme_style));
    frame.render_widget(theme_field, chunks[1]);

    let path_style = if app.tab_index == 6 && app.settings_option == SettingsOption::InstallPath { Style::default().fg(theme.warning).bold() } else { Style::default() };
    let path = Paragraph::new(app.config.install_path.as_str())
        .block(Block::default().borders(Borders::ALL).title(" 📁 MAESTRO INSTALL PATH ").border_style(path_style));
    frame.render_widget(path, chunks[2]);

    let save_style = if app.tab_index == 6 && app.settings_option == SettingsOption::Save { Style::default().bg(theme.success).fg(Color::Black).bold() } else { Style::default().fg(theme.success) };
    let save = Paragraph::new(" [ SAVE CONFIGURATION ] ")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(save_style));
    frame.render_widget(save, chunks[3]);

    let help = Paragraph::new("Use ↑/↓ to navigate, Enter to edit selected field. Settings are stored in ~/.config/maestro/config.toml")
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.muted));
    frame.render_widget(help, chunks[4]);
}

fn render_new_project_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 40, frame.area());
    frame.render_widget(Clear, area);
    
    let step = match app.input_mode {
        InputMode::NewProjectName => 1,
        InputMode::NewProjectPath => 2,
        InputMode::NewProjectTool => 3,
        _ => 1,
    };

    let block = Block::default()
        .title(format!(" NEW PROJECT WIZARD (Step {} of 3) ", step))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(15, 10, 20)));
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Name
            Constraint::Length(3), // Path
            Constraint::Length(3), // Tool
            Constraint::Min(0),    // Help/Hint
        ])
        .split(area);

    let name_style = if step == 1 { Style::default().fg(Color::Yellow).bold() } else { Style::default().fg(Color::DarkGray) };
    let name = Paragraph::new(app.new_project_name.as_str())
        .block(Block::default().borders(Borders::ALL).title(" 1. PROJECT NAME ").border_style(name_style));
    frame.render_widget(name, chunks[0]);

    let path_style = if step == 2 { Style::default().fg(Color::Cyan).bold() } else { Style::default().fg(Color::DarkGray) };
    let path = Paragraph::new(app.new_project_path.as_str())
        .block(Block::default().borders(Borders::ALL).title(" 2. TARGET PATH (Enter for current) ").border_style(path_style));
    frame.render_widget(path, chunks[1]);

    let tool_style = if step == 3 { Style::default().fg(Color::Magenta).bold() } else { Style::default().fg(Color::DarkGray) };
    let tool = Paragraph::new(app.new_project_tool.as_str())
        .block(Block::default().borders(Borders::ALL).title(" 3. INITIAL TOOL (None/claude/gemini) ").border_style(tool_style));
    frame.render_widget(tool, chunks[2]);

    let hint = Paragraph::new("Press 'Enter' to confirm step, 'Esc' to cancel\n\nThis will run /maestro:setup in the target directory.")
        .alignment(Alignment::Center);
    frame.render_widget(hint, chunks[3]);
    frame.render_widget(block, area);
}

fn render_group_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 40, frame.area());
    frame.render_widget(Clear, area);
    
    let step = match app.input_mode {
        InputMode::NewGroupTitle | InputMode::RenameGroup => 1,
        InputMode::NewGroupCategory | InputMode::RenameGroupCategory => 2,
        _ => 1,
    };

    let title = if matches!(app.input_mode, InputMode::RenameGroup | InputMode::RenameGroupCategory) {
        " RENAME GROUP WIZARD "
    } else {
        " NEW GROUP WIZARD "
    };

    let block = Block::default()
        .title(format!(" {} (Step {} of 2) ", title, step))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(10, 20, 15)));
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Name
            Constraint::Length(3), // Category
            Constraint::Min(0),    // Help/Hint
        ])
        .split(area);

    let name_style = if step == 1 { Style::default().fg(Color::Yellow).bold() } else { Style::default().fg(Color::DarkGray) };
    let name = Paragraph::new(app.rename_buffer.as_str())
        .block(Block::default().borders(Borders::ALL).title(" 1. GROUP NAME ").border_style(name_style));
    frame.render_widget(name, chunks[0]);

    let cat_style = if step == 2 { Style::default().fg(Color::Cyan).bold() } else { Style::default().fg(Color::DarkGray) };
    let cat = Paragraph::new(app.new_group_category.as_str())
        .block(Block::default().borders(Borders::ALL).title(" 2. CATEGORY (e.g. Work, Personal, Research) ").border_style(cat_style));
    frame.render_widget(cat, chunks[1]);

    let hint = Paragraph::new("Tab to switch fields, Enter: next/save, Esc to cancel\n\nGroups help you organize your coding sessions.")
        .alignment(Alignment::Center);
    frame.render_widget(hint, chunks[2]);
    frame.render_widget(block, area);
}

fn render_new_track_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 30, frame.area());
    frame.render_widget(Clear, area);
    
    let step = match app.input_mode {
        InputMode::NewTrackTitle => 1,
        InputMode::NewTrackType => 2,
        _ => 1,
    };

    let block = Block::default()
        .title(format!(" NEW TRACK WIZARD (Step {} of 2) ", step))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(10, 15, 20)));
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Type
            Constraint::Min(0),    // Help/Hint
        ])
        .split(area);

    let title_style = if step == 1 { Style::default().fg(Color::Yellow).bold() } else { Style::default().fg(Color::DarkGray) };
    let title = Paragraph::new(app.new_track_title.as_str())
        .block(Block::default().borders(Borders::ALL).title(" 1. TRACK TITLE ").border_style(title_style));
    frame.render_widget(title, chunks[0]);

    let type_style = if step == 2 { Style::default().fg(Color::Cyan).bold() } else { Style::default().fg(Color::DarkGray) };
    let type_text = if app.new_track_is_master { "[X] Master Track  [ ] Direct Track" } else { "[ ] Master Track  [X] Direct Track" };
    let track_type = Paragraph::new(type_text)
        .block(Block::default().borders(Borders::ALL).title(" 2. TRACK TYPE (Space to toggle) ").border_style(type_style));
    frame.render_widget(track_type, chunks[1]);

    let hint = Paragraph::new("Press 'Enter' to confirm, 'Esc' to cancel\n\nThis will run /maestro:newTrack in the project.")
        .alignment(Alignment::Center);
    frame.render_widget(hint, chunks[2]);
    frame.render_widget(block, area);
}
fn render_input_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, frame.area());
    let block = Block::default()
        .title(" New Session Wizard ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));

    let mut text = vec![Line::from("")];
    
    // Title Field
    let title_style = if app.input_mode == InputMode::NewSessionTitle { Style::default().fg(Color::Yellow).bold() } else { Style::default() };
    text.push(Line::from(vec![
        Span::styled("  Session Title: ", title_style),
        Span::raw(&app.new_session_title),
        if app.input_mode == InputMode::NewSessionTitle { Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)) } else { Span::raw("") },
    ]));

    // Path Field
    let path_style = if app.input_mode == InputMode::NewSessionPath { Style::default().fg(Color::Yellow).bold() } else { Style::default() };
    text.push(Line::from(vec![
        Span::styled("  Project Path:  ", path_style),
        Span::raw(&app.new_session_path),
        if app.input_mode == InputMode::NewSessionPath { Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)) } else { Span::raw("") },
    ]));

    // Tool Field
    let tool_style = if app.input_mode == InputMode::NewSessionTool { Style::default().fg(Color::Yellow).bold() } else { Style::default() };
    text.push(Line::from(vec![
        Span::styled("  Tool (Cycle):  ", tool_style),
        Span::styled(&app.new_session_tool, Style::default().fg(Color::Cyan).bold()),
        if app.input_mode == InputMode::NewSessionTool { Span::raw(" (Press any key to cycle)") } else { Span::raw("") },
    ]));

    text.push(Line::from(""));
    text.push(Line::from("  [Enter] Next/Confirm  [Esc] Cancel"));

    let para = Paragraph::new(text).block(block);
    frame.render_widget(Clear, area);
    frame.render_widget(para, area);
}

fn render_switcher_modal(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(50, 40, frame.area());
    let theme = app.theme();
    let block = Block::default()
        .title(" Quick Session Switcher ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(theme.panel_bg));

    if app.sessions.is_empty() {
        let text = vec![Line::from("  No active sessions.")];
        let para = Paragraph::new(text).block(block);
        frame.render_widget(Clear, area);
        frame.render_widget(para, area);
    } else {
        let items: Vec<ListItem> = app.sessions.iter().map(|s| {
            let status_color = if s.status == leindex_analyzers::memory::models::SessionStatus::Running { Color::Green } else { Color::Gray };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(" * ", Style::default().fg(status_color)),
                    Span::styled(&s.title, Style::default().bold().fg(Color::White)),
                    Span::styled(format!(" [{}]", s.tool.as_deref().unwrap_or("?")), Style::default().fg(Color::DarkGray)),
                ]),
            ])
        }).collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(theme.highlight_bg).fg(theme.highlight_fg).bold())
            .highlight_symbol(">> ");
        
        frame.render_widget(Clear, area);
        frame.render_stateful_widget(list, area, &mut app.switcher_state);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn session_log_tail(session_name: &str, lines: usize) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let path = format!("{}/.maestro/logs/{}.log", home, session_name);

    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();

    // Avoid loading the entire file: read only the tail window.
    let window: u64 = 128 * 1024;
    let start = len.saturating_sub(window);
    let _ = file.seek(SeekFrom::Start(start)).ok()?;

    let mut buf = String::new();
    file.read_to_string(&mut buf).ok()?;

    let mut out: Vec<String> = Vec::new();
    for line in buf.lines().rev().take(lines) {
        out.push(line.to_string());
    }
    out.reverse();
    Some(out.join("\n"))
}

fn render_projects(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 🚀 Projects ")
        .title_style(Style::default().fg(Color::Cyan));

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    if app.projects.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from("  No projects indexed."),
            Line::from(""),
            Line::from("  Run \"maestro scan\" to find projects."),
        ];
        let para = Paragraph::new(text).block(block);
        frame.render_widget(para, area);
    } else {
        // Project List (Left)
        let items: Vec<ListItem> = app.projects.iter().map(|p| {
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("  {} ", "📦"), Style::default()),
                    Span::styled(&p.name, Style::default().fg(Color::Cyan).bold()),
                ]),
                Line::from(vec![
                    Span::styled("    ", Style::default()),
                    Span::styled(&p.path, Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(""),
            ])
        }).collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::Rgb(30, 30, 50)).fg(Color::Yellow).bold())
            .highlight_symbol(">> ");
        frame.render_stateful_widget(list, chunks[0], &mut app.project_state);

        // File Preview / "Yazi" Column (Right)
        let preview_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(if app.preview_focused { " 📂 File Explorer (Focused) " } else { " 📂 File Explorer " })
            .border_style(if app.preview_focused { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) });

        if let Some(i) = app.project_state.selected() {
            let project = &app.projects[i];
            let current_path = app.project_explorer_path.clone().unwrap_or_else(|| project.path.clone());
            let expanded_path = if current_path.starts_with('~') {
                current_path.replacen('~', &std::env::var("HOME").unwrap_or_default(), 1)
            } else {
                current_path.clone()
            };





            // List directory contents
            let mut file_items = Vec::new();

            if let Ok(entries) = std::fs::read_dir(&expanded_path) {
                let mut dir_entries: Vec<_> = entries.flatten().collect();
                dir_entries.sort_by_key(|e| (!e.path().is_dir(), e.file_name()));

                app.explorer_items = dir_entries.iter().map(|e| e.file_name().to_string_lossy().to_string()).collect();

                for (idx, entry) in dir_entries.iter().enumerate().take(30) {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry.path().is_dir();
                    let icon = if is_dir { "📁" } else { "📄" };
                    let color = if is_dir { Color::Blue } else { Color::White };
                    
                    let style = if app.preview_focused && idx == app.project_explorer_selected {
                        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(color)
                    };

                    file_items.push(ListItem::new(Line::from(vec![
                        Span::styled(format!("    {} ", icon), style),
                        Span::styled(file_name, style),
                    ])));
                }
                if dir_entries.len() > 30 {
                    file_items.push(ListItem::new(Line::from(vec![
                        Span::styled(format!("    ... and {} more items", dir_entries.len() - 30), Style::default().fg(Color::DarkGray).italic()),
                    ])));
                }
            } else {
                file_items.push(ListItem::new(Span::styled("  Error reading directory. (Path might not exist or need expansion)", Style::default().fg(Color::Red))));
            }
            
            let list = List::new(file_items).block(preview_block);
            frame.render_widget(list, chunks[1]);
        }
 else {
            let para = Paragraph::new(vec![
                Line::from(""),
                Line::from("  Select a project to explore its files."),
                Line::from(""),
                Line::from("  Press Enter to open in:"),
                 Line::from(vec![
                    Span::styled(format!("  {} ", app.config.editor.to_uppercase()), Style::default().fg(Color::Green).bold()),
                ]),
                Line::from(""),
                Line::from("  (Use 'Space' on installer to change editor)"),
            ]).block(preview_block).alignment(Alignment::Center);
            frame.render_widget(para, chunks[1]);
        }
    }
}

fn render_memory(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 🔎 Memory Search (Ctrl+F, Ctrl+L clear, r refresh) ")
        .title_style(Style::default().fg(theme.accent));

    let search_text = if app.input_mode == InputMode::MemorySearch {
        format!("{}█", app.memory_query)
    } else {
        app.memory_query.clone()
    };
    frame.render_widget(Paragraph::new(search_text).block(search_block), chunks[0]);

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 🧠 Memory Results ")
        .title_style(Style::default().fg(theme.accent_alt));

    if app.memories.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from("  No memories found."),
            Line::from(""),
            Line::from("  Tip: press 'r' to import system-wide memories."),
        ];
        let para = Paragraph::new(text).block(list_block);
        frame.render_widget(para, chunks[1]);
        return;
    }

    let items: Vec<ListItem> = app
        .memories
        .iter()
        .map(|m| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("[{}] ", m.category), Style::default().fg(Color::Yellow)),
                Span::styled(m.content.clone(), Style::default().fg(Color::White)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(list_block)
        .highlight_style(Style::default().bg(theme.highlight_bg).fg(theme.highlight_fg).bold())
        .highlight_symbol(">> ");
    frame.render_stateful_widget(list, chunks[1], &mut app.memory_state);
}

fn render_lsps(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();

    // Collect missing LSPs for installation guidance
    let missing_lsps: Vec<&str> = app.lsp_availability
        .iter()
        .filter(|(_, &available)| !available)
        .map(|(name, _)| name.as_str())
        .collect();

    // Determine if we need to show the missing LSPs section
    let has_missing_lsps = !missing_lsps.is_empty();

    // Calculate constraints: header + LSP list + (optional) missing LSPs section
    let list_min = if has_missing_lsps {
        // Reserve space for missing LSPs section (approximately 15 lines)
        Constraint::Min(0)
    } else {
        Constraint::Min(0)
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            list_min,               // LSP list
            Constraint::Length(if has_missing_lsps { 15 } else { 0 }),  // Missing LSPs section
        ])
        .split(area);

    // Header block with control hints
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 🔌 Language Server Protocol (LSP) Status ")
        .title_style(Style::default().fg(theme.accent));

    let header_text = vec![
        Line::from(vec![
            Span::styled("Controls: ", Style::default().fg(theme.muted)),
            Span::styled("[s] Toggle ", Style::default().fg(theme.warning).bold()),
            Span::styled("| ", Style::default().fg(theme.muted)),
            Span::styled("[R] Restart ", Style::default().fg(theme.warning).bold()),
            Span::styled("| ", Style::default().fg(theme.muted)),
            Span::styled("[r] Refresh ", Style::default().fg(theme.warning).bold()),
            Span::styled("| ", Style::default().fg(theme.muted)),
            Span::styled("[l] Logs", Style::default().fg(theme.warning).bold()),
        ]),
    ];
    frame.render_widget(Paragraph::new(header_text).block(header_block), chunks[0]);

    // Collect all LSPs across all sessions into a flat list
    let mut lsp_entries: Vec<(String, String, LspStatus, Option<String>)> = Vec::new();
    // (session_id, lsp_name, status, session_title)

    for session in &app.sessions {
        let session_title = session.title.clone();
        if let Some(lsp_states) = app.lsp_status_cache.get(&session.session_id) {
            for (lsp_name, status) in lsp_states {
                lsp_entries.push((
                    session.session_id.clone(),
                    lsp_name.clone(),
                    *status,
                    Some(session_title.clone()),
                ));
            }
        }
    }

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 📋 LSP Servers (by session) ")
        .title_style(Style::default().fg(theme.accent_alt));

    if lsp_entries.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from("  No LSP servers found."),
            Line::from(""),
            Line::from("  Tip: LSPs are auto-detected from tmux sessions."),
            Line::from("  Press 'r' to refresh status."),
        ];
        let para = Paragraph::new(text).block(list_block);
        frame.render_widget(para, chunks[1]);
    } else {
        // Create list items with color-coded status
        let lsp_items: Vec<ListItem> = lsp_entries
            .iter()
            .map(|(_session_id, lsp_name, status, session_title)| {
                let (status_text, status_color, icon) = match status {
                    LspStatus::Running => ("Running", Color::Green, "●"),
                    LspStatus::Stopped => ("Stopped", Color::Red, "■"),
                    LspStatus::Error => ("Error", Color::Red, "⚠"),
                    LspStatus::Starting => ("Starting", Color::Yellow, "○"),
                };

                // Get short session title (truncate if too long)
                // Use character-based slicing to avoid UTF-8 truncation panic
                let short_title = session_title.as_ref().map(|t| {
                    if t.chars().count() > 20 {
                        let truncated: String = t.chars().take(17).collect();
                        format!("{}...", truncated)
                    } else {
                        t.clone()
                    }
                }).unwrap_or_else(|| "Unknown".to_string());

                ListItem::new(Line::from(vec![
                    Span::styled(icon, Style::default().fg(status_color)),
                    Span::raw(" "),
                    Span::styled(format!("{} ", lsp_name), Style::default().bold()),
                    Span::styled(
                        format!("[{}] ", status_text),
                        Style::default().fg(status_color),
                    ),
                    Span::styled(
                        format!("({})", short_title),
                        Style::default().fg(Color::DarkGray).italic(),
                    ),
                ]))
            })
            .collect();

        let lsp_list = List::new(lsp_items)
            .block(list_block)
            .highlight_style(Style::default().bg(theme.highlight_bg).fg(theme.highlight_fg).bold())
            .highlight_symbol(">> ");
        frame.render_stateful_widget(lsp_list, chunks[1], &mut app.lsp_state);
    }

    // Missing LSPs section with installation commands
    if has_missing_lsps {
        let missing_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" ⚠ Missing LSPs - Installation Required ")
            .title_style(Style::default().fg(Color::Yellow).bold());

        let mut missing_lines = vec![
            Line::from(vec![
                Span::styled("The following LSPs are not available on your system:", Style::default().fg(Color::Yellow).bold()),
            ]),
            Line::from(""),
        ];

        for lsp_name in &missing_lsps {
            let install_commands = App::get_lsp_install_command(lsp_name);
            missing_lines.push(Line::from(vec![
                Span::styled(
                    format!("▸ {} ", lsp_name),
                    Style::default().fg(Color::Red).bold(),
                ),
                Span::styled(
                    "NOT FOUND",
                    Style::default().fg(Color::Red).bold(),
                ),
            ]));

            for cmd in &install_commands {
                if cmd.starts_with("#") {
                    missing_lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(*cmd, Style::default().fg(Color::DarkGray).italic()),
                    ]));
                } else if !cmd.is_empty() {
                    missing_lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("$ {}", cmd),
                            Style::default().fg(Color::Cyan),
                        ),
                    ]));
                } else {
                    missing_lines.push(Line::from(""));
                }
            }
            missing_lines.push(Line::from(""));
        }

        let missing_para = Paragraph::new(missing_lines)
            .block(missing_block)
            .wrap(Wrap { trim: false });
        frame.render_widget(missing_para, chunks[2]);
    }
}

fn render_analysis(frame: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),      // History
            Constraint::Length(3),   // Progress / Status
            Constraint::Length(3),   // Input Prompt
        ])
        .split(area);

    let theme = app.theme();
    let hub_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 🚀 Analysis Command Hub ")
        .title_style(Style::default().fg(theme.accent_alt));

    // History View
    let mut history_lines = vec![
        Line::from(vec![
            Span::styled(" Maestro Analysis Engine v2.0 READY", Style::default().fg(Color::Green).bold()),
        ]),
        Line::from(vec![
            Span::styled(" Type '/phase1 <path>' to begin. ", Style::default().fg(Color::Gray)),
            Span::styled("(Press 'a' to enter Command Hub)", Style::default().fg(Color::DarkGray).italic()),
        ]),
        Line::from(""),
    ];

    let examples = vec![
        Line::from(vec![Span::styled(" EXAMPLES:", Style::default().fg(Color::Yellow).bold())]),
        Line::from("  $ /phase1 . --mode ultra --files 20"),
        Line::from("  $ /phase2 ."),
        Line::from("  $ /phase3 . --focus-files 2"),
        Line::from("  $ /phase4 . --top 10"),
        Line::from("  $ /phase5 ."),
        Line::from(""),
    ];
    history_lines.extend(examples);

    if app.analysis_history.is_empty() {
        history_lines.push(Line::from("  No recent analysis runs."));
    } else {
        for line in &app.analysis_history {
            history_lines.push(Line::from(line.as_str()));
        }
    }

    let history = Paragraph::new(history_lines)
        .block(hub_block)
        .wrap(Wrap { trim: true });
    frame.render_widget(history, chunks[0]);

    // Progress / Status Bar
    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let status_text = if app.input_mode == InputMode::AnalysisPrompt {
        " STATUS: Awaiting Command... "
    } else {
        " STATUS: Idle "
    };
    let status = Paragraph::new(status_text)
        .block(status_block)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(status, chunks[1]);

    // Input Prompt
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(if app.input_mode == InputMode::AnalysisPrompt { " ⌨️ Command (Esc/Enter to finish) > " } else { " Command > " })
        .title_style(Style::default().fg(Color::Cyan));
    
    let input_text = if app.input_mode == InputMode::AnalysisPrompt {
        format!("{}█", app.analysis_input)
    } else {
        app.analysis_input.clone()
    };
    
    let input = Paragraph::new(input_text)
        .block(input_block)
        .style(if app.input_mode == InputMode::AnalysisPrompt { 
            Style::default().fg(Color::Yellow) 
        } else { 
            Style::default().fg(Color::Gray) 
        });
    frame.render_widget(input, chunks[2]);
}

fn render_sessions(frame: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(if !app.preview_focused { BorderType::Double } else { BorderType::Rounded })
        .title(" 📁 Sessions & Groups ")
        .title_style(if !app.preview_focused { Style::default().fg(Color::Cyan).bold() } else { Style::default().fg(Color::DarkGray) });

    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_type(if app.preview_focused { BorderType::Double } else { BorderType::Rounded })
        .title(format!(" 🖥️ Preview {} ", if app.preview_focused { "[FOCUSED]" } else { "" }))
        .title_style(if app.preview_focused { Style::default().fg(Color::Yellow).bold() } else { Style::default().fg(Color::DarkGray) });

    if app.session_entries.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from("  No sessions or groups found."),
            Line::from(""),
            Line::from("  Create a new session with 'n' or select a project."),
        ];
        let para = Paragraph::new(text).block(list_block);
        frame.render_widget(para, area);
    } else {
        let mut items = Vec::new();

        // Pre-collect LSP indicators for all sessions (single borrow, then release)
        // Clone the cache first to avoid borrow checker issues
        let lsp_cache = app.lsp_status_cache.clone();
        let lsp_indicators_map = {
            let mut map = std::collections::HashMap::new();
            for (session_id, lsps) in &lsp_cache {
                if !lsps.is_empty() {
                    let indicators: Vec<Span> = lsps
                        .iter()
                        .map(|(lsp_name, status)| {
                            let (icon, color) = match status {
                                LspStatus::Running => (" ● ", Color::Green),
                                LspStatus::Starting => (" ◐ ", Color::Yellow),
                                LspStatus::Error => (" x ", Color::Red),
                                LspStatus::Stopped => (" ○ ", Color::Gray),
                            };
                            let short_name = if lsp_name.contains("rust") {
                                "R"
                            } else if lsp_name.contains("ruff") || lsp_name.contains("python") {
                                "P"
                            } else if lsp_name.contains("typescript") || lsp_name.contains("ts") {
                                "T"
                            } else {
                                "?"
                            };
                            Span::styled(
                                format!("{}{}", short_name, icon),
                                Style::default().fg(color),
                            )
                        })
                        .collect();
                    map.insert(session_id.clone(), indicators);
                }
            }
            map
        };
        // `app` borrow is now released

        for (i, entry) in app.session_entries.iter().enumerate() {
            match entry {
                SessionEntry::Group(g) => {
                    let icon = if g.is_expanded { "▼ " } else { "▶ " };
                    items.push(ListItem::new(vec![
                        Line::from(vec![
                            Span::styled(format!("  {} ", icon), Style::default().fg(Color::Yellow)),
                            Span::styled(&g.name, Style::default().bold().fg(Color::White)),
                            Span::styled(
                                if let Some(cat) = &g.category { format!(" [{}]", cat) } else { "".to_string() },
                                Style::default().fg(Color::Cyan)
                            ),
                            Span::styled(format!(" ({})", g.path), Style::default().fg(Color::DarkGray).italic()),
                        ]),
                    ]));
                }
                SessionEntry::Session(s) => {
                    let is_running = s.status == leindex_analyzers::memory::models::SessionStatus::Running;
                    let is_terminated = s.status == leindex_analyzers::memory::models::SessionStatus::Terminated;
                    let is_waiting = s.status == leindex_analyzers::memory::models::SessionStatus::Waiting;

                    let (status_icon, status_color) = if is_running {
                        (" * ", Color::Green)
                    } else if is_terminated {
                        (" x ", Color::Red)
                    } else if is_waiting {
                        (" ◒ ", Color::Yellow)
                    } else {
                        (" o ", Color::Gray)
                    };

                    let title_style = if is_running {
                        Style::default().fg(Color::Cyan)
                    } else if is_terminated {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    // Determine if this is the last item in a group (for L-line)
                    let mut branch = " ├─";
                    let is_last_in_group = if let Some(next) = app.session_entries.get(i + 1) {
                        matches!(next, SessionEntry::Group(_))
                    } else {
                        // End of list is also end of group
                        true
                    };

                    if is_last_in_group {
                        branch = " └─";
                    }

                    let mut line_spans = vec![
                        Span::styled(format!("  {}", branch), Style::default().fg(Color::DarkGray)),
                        Span::styled(status_icon, Style::default().fg(status_color)),
                        Span::styled(&s.title, title_style),
                    ];

                    if s.status == leindex_analyzers::memory::models::SessionStatus::Terminated {
                        line_spans.push(Span::styled(" [KILLED]", Style::default().fg(Color::Red).bold()));
                    }

                    line_spans.push(Span::styled(format!(" [{}]", s.tool.as_deref().unwrap_or("?")), Style::default().fg(Color::DarkGray)));

                    // Add LSP indicators if any exist (use pre-collected map)
                    if let Some(indicators) = lsp_indicators_map.get(&s.session_id) {
                        if !indicators.is_empty() {
                            line_spans.push(Span::raw(" "));
                            for indicator in indicators {
                                line_spans.push(indicator.clone());
                            }
                        }
                    }

                    items.push(ListItem::new(vec![Line::from(line_spans)]));
                }
            }
        }

        let list = List::new(items)
            .block(list_block)
            .highlight_style(Style::default().bg(Color::Rgb(40, 40, 60)).fg(Color::White).bold());
        frame.render_stateful_widget(list, chunks[0], &mut app.session_state);

        // Render Preview
        let mut preview_lines = Vec::new();

        if let Some(i) = app.session_state.selected() {
            if let Some(SessionEntry::Session(s)) = app.session_entries.get(i) {
                // Header (Replicating Go TUI)
                let status_icon = match s.status {
                    leindex_analyzers::memory::models::SessionStatus::Running => "●",
                    leindex_analyzers::memory::models::SessionStatus::Waiting => "◐",
                    _ => "○",
                };
                let status_color = match s.status {
                    leindex_analyzers::memory::models::SessionStatus::Running => Color::Green,
                    leindex_analyzers::memory::models::SessionStatus::Waiting => Color::Yellow,
                    leindex_analyzers::memory::models::SessionStatus::Terminated => Color::Red,
                    _ => Color::DarkGray,
                };

                // Row 1: Icon Title (ID)
                preview_lines.push(Line::from(vec![
                    Span::styled(format!(" {} ", status_icon), Style::default().fg(status_color).bold()),
                    Span::styled(&s.title, Style::default().fg(Color::Cyan).bold()),
                    Span::styled(format!(" ({})", s.session_id), Style::default().fg(Color::DarkGray)),
                ]));

                // Row 2: Tool, Group, Activity
                let activity_str = "active now"; // Placeholder, replace with actual activity logic if available
                preview_lines.push(Line::from(vec![
                    Span::styled(format!(" {} ", s.tool.as_deref().unwrap_or("shell")), 
                        Style::default().bg(Color::Magenta).fg(Color::Black)),
                    Span::raw(" "),
                    Span::styled(format!(" {} ", s.group_path.as_deref().unwrap_or("Uncategorized")), 
                        Style::default().bg(Color::Cyan).fg(Color::Black)),
                    Span::raw(" "),
                    Span::styled(format!(" ⏱ {}", activity_str), Style::default().fg(Color::DarkGray)),
                ]));

                // Row 3: Path
                preview_lines.push(Line::from(vec![
                    Span::styled(" 📁 ", Style::default()),
                    Span::styled(&s.project_path, Style::default().fg(Color::DarkGray)),
                ]));

	                // Row 4: Tool session IDs (best-effort capture)
	                if let Some(ref metadata) = s.metadata {
	                    if s.tool.as_deref() == Some("claude") {
	                        if let Some(cid) = metadata.get("claude_session_id").and_then(|v| v.as_str()) {
	                            preview_lines.push(Line::from(vec![
	                                Span::styled(" Claude: ", Style::default().fg(Color::DarkGray)),
	                                Span::styled("● Connected", Style::default().fg(Color::Green)),
	                            ]));
	                            preview_lines.push(Line::from(vec![
	                                Span::styled(" Session ID: ", Style::default().fg(Color::DarkGray)),
	                                Span::styled(cid, Style::default().fg(Color::White)),
	                            ]));
	                        }
	                    }
	                    if s.tool.as_deref() == Some("gemini") {
	                        if let Some(gid) = metadata.get("gemini_session_id").and_then(|v| v.as_str()) {
	                            preview_lines.push(Line::from(vec![
	                                Span::styled(" Gemini: ", Style::default().fg(Color::DarkGray)),
	                                Span::styled("● Connected", Style::default().fg(Color::Green)),
	                            ]));
	                            preview_lines.push(Line::from(vec![
	                                Span::styled(" Session ID: ", Style::default().fg(Color::DarkGray)),
	                                Span::styled(gid, Style::default().fg(Color::White)),
	                            ]));
	                        }
	                    }
	                    if s.tool.as_deref() == Some("codex") {
	                        if let Some(cid) = metadata.get("codex_session_id").and_then(|v| v.as_str()) {
	                            preview_lines.push(Line::from(vec![
	                                Span::styled(" Codex: ", Style::default().fg(Color::DarkGray)),
	                                Span::styled("● Captured", Style::default().fg(Color::Green)),
	                            ]));
	                            preview_lines.push(Line::from(vec![
	                                Span::styled(" Session ID: ", Style::default().fg(Color::DarkGray)),
	                                Span::styled(cid, Style::default().fg(Color::White)),
	                            ]));
	                        }
	                    }
	                    if s.tool.as_deref() == Some("opencode") {
	                        if let Some(oid) = metadata.get("opencode_session_id").and_then(|v| v.as_str()) {
	                            preview_lines.push(Line::from(vec![
	                                Span::styled(" OpenCode: ", Style::default().fg(Color::DarkGray)),
	                                Span::styled("● Captured", Style::default().fg(Color::Green)),
	                            ]));
	                            preview_lines.push(Line::from(vec![
	                                Span::styled(" Session ID: ", Style::default().fg(Color::DarkGray)),
	                                Span::styled(oid, Style::default().fg(Color::White)),
	                            ]));
	                        }
	                    }
	                    if s.tool.as_deref() == Some("amp") {
	                        if let Some(tid) = metadata.get("amp_thread_id").and_then(|v| v.as_str()) {
	                            preview_lines.push(Line::from(vec![
	                                Span::styled(" Amp: ", Style::default().fg(Color::DarkGray)),
	                                Span::styled("● Captured", Style::default().fg(Color::Green)),
	                            ]));
	                            preview_lines.push(Line::from(vec![
	                                Span::styled(" Thread ID: ", Style::default().fg(Color::DarkGray)),
	                                Span::styled(tid, Style::default().fg(Color::White)),
	                            ]));
	                        }
	                    }

	                    if let Some(mcps) = metadata.get("loaded_mcp_names").and_then(|v| v.as_array()) {
	                        let mcp_names: Vec<String> = mcps
	                            .iter()
	                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                        if !mcp_names.is_empty() {
                            preview_lines.push(Line::from(vec![
                                Span::styled(" 🔌 MCPs: ", Style::default().fg(Color::Cyan)),
                                Span::styled(mcp_names.join(", "), Style::default().fg(Color::White)),
                            ]));
                        }
                    }
                }

                if s.tool.as_deref() == Some("claude") {
                    preview_lines.push(Line::from(vec![
                        Span::styled(" Fork: ", Style::default().fg(Color::DarkGray).italic()),
                        Span::styled("f ", Style::default().fg(Color::Cyan).bold()),
                        Span::raw("(quick), "),
                        Span::styled("F ", Style::default().fg(Color::Cyan).bold()),
                        Span::raw("(options)"),
                    ]));
                }

                // Divider
                preview_lines.push(Line::from(""));
                let divider_width = (chunks[1].width as usize).saturating_sub(6);
                let divider = "─".repeat(divider_width / 2 - 4);
                preview_lines.push(Line::from(vec![
                    Span::styled(format!(" {} Output {} ", divider, divider), Style::default().fg(Color::DarkGray)),
                ]));
                preview_lines.push(Line::from(""));
            }
        }

        if app.session_preview_content.is_empty() {
            preview_lines.push(Line::from("  (No preview available)"));
        } else {
            for line in app.session_preview_content.lines() {
                preview_lines.push(Line::from(format!("  {}", line)));
            }
        }
        
        let preview = Paragraph::new(preview_lines)
            .block(preview_block)
            .wrap(Wrap { trim: false })
            .scroll((app.preview_scroll, 0));
        frame.render_widget(preview, chunks[1]);
    }
}
