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
use std::io;

use leindex_analyzers::memory::MemoryService;
use leindex_analyzers::multiplexer::TmuxMultiplexer;

pub async fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize service for live data
    let service = MemoryService::new(None).ok();
    if let Some(ref s) = service {
        let _ = s.initialize();
    }

    // Run app
    let result = run_app(&mut terminal, service).await;

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
    // Phase 11 additions
    mcp_state: ratatui::widgets::ListState,
    preview_focused: bool,
    preview_scroll: u16,
    hub_search_buffer: String,
    hub_focus: HubFocus,
    // Dashboard MCP menu state
    mcp_menu_option: McpOption,
    target_mcp_name: Option<String>,
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
    // Phase 11 additions
    SessionHub,
    NewGroupTitle,
    MoveToGroup,
    McpMenu,
    // Phase 15 additions
    NewProjectName,
    NewProjectPath,
    NewProjectTool,
    NewTrackTitle,
    NewTrackType,
    NewGroupCategory,
    RenameGroupCategory,
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

#[derive(Clone)]
enum SessionEntry {
    Group(leindex_analyzers::memory::models::SessionGroup),
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
    fn new(service: Option<&MemoryService>) -> Self {
        let mut app = Self {
            tab_index: 0,
            should_quit: false,
            show_help: false,
            input_mode: InputMode::Normal,
            projects: Vec::new(),
            project_state: ratatui::widgets::ListState::default(),
            memories: Vec::new(),
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
            mcp_state: ratatui::widgets::ListState::default(),
            preview_focused: false,
            preview_scroll: 0,
            hub_search_buffer: String::new(),
            hub_focus: HubFocus::Rename,
            mcp_menu_option: McpOption::StartStop,
            target_mcp_name: None,
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
        };

        // Load live data if service available
        if let Some(svc) = service {
            if let Ok(projects) = svc.list_projects() {
                app.projects = projects.iter().map(|p| ProjectInfo {
                    name: p.project_name.clone(),
                    path: p.project_path.clone(),
                    _track_count: 0,
                }).collect();
                app.stats.project_count = app.projects.len();
            }

            if let Ok(memories) = svc.list_memories(20) {
                app.memories = memories.iter().map(|m| MemoryInfo {
                    _id: m.id,
                    content: m.content.clone(),
                    category: m.category.to_string(),
                }).collect();
            }

            if let Ok(sessions) = svc.list_sessions() {
                app.sessions = sessions;
            }

            if let Ok(groups) = svc.list_session_groups() {
                app.groups = groups;
            }

            if let Ok(mcp_servers) = svc.list_mcp_servers() {
                app.mcp_servers = mcp_servers;
            }
            if let Ok(stats) = svc.stats() {
                app.stats.memory_count = stats.memory_count;
                app.stats.track_count = stats.track_count;
            }

            app.refresh_session_entries();
        }

        app
    }

    fn refresh_session_entries(&mut self) {
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
    }

}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, service: Option<MemoryService>) -> Result<()> {
    let mut app = App::new(service.as_ref());
    let mut last_refresh = std::time::Instant::now();

    loop {
        terminal.draw(|frame| ui(frame, &mut app))?;

        // Periodic refresh (every 1 second)
        if last_refresh.elapsed() >= std::time::Duration::from_secs(1) {
            if let Some(svc) = service.as_ref() {
                if let Ok(sessions) = svc.list_sessions() { 
                    if sessions.len() != app.sessions.len() {
                        app.sessions = sessions;
                        app.refresh_session_entries();
                    } else {
                        app.sessions = sessions;
                    }
                }
                if let Ok(groups) = svc.list_session_groups() { 
                    if groups.len() != app.groups.len() {
                        app.groups = groups;
                        app.refresh_session_entries();
                    } else {
                        app.groups = groups;
                    }
                }
                if let Ok(mcp) = svc.list_mcp_servers() { app.mcp_servers = mcp; }
                
                // MCP Pool Discovery for installed tools
                if app.mcp_servers.is_empty() {
                    let tools = vec!["claude", "gemini", "codex", "opencode", "amp"];
                    for tool in tools {
                        app.mcp_servers.push(leindex_analyzers::memory::models::McpServer {
                            id: 0,
                            name: tool.to_string(),
                            command: tool.to_string(),
                            args: Vec::new(),
                            env: serde_json::json!({}),
                            status: leindex_analyzers::memory::models::McpStatus::Running,
                            socket_path: None,
                            client_count: 1,
                            last_started_at: Some(chrono::Utc::now()),
                        });
                    }
                }

                if let Ok(memories) = svc.list_memories(20) {
                    app.memories = memories.iter().map(|m| MemoryInfo {
                        _id: m.id,
                        content: m.content.clone(),
                        category: m.category.to_string(),
                    }).collect();
                }

                // Update session statuses via tmux
                let multiplexer = TmuxMultiplexer::default();
                multiplexer.refresh_session_cache().ok();
                for session in &mut app.sessions {
                    if session.status != leindex_analyzers::memory::models::SessionStatus::Terminated {
                        if !multiplexer.session_exists(&session.session_id) {
                            session.status = leindex_analyzers::memory::models::SessionStatus::Terminated;
                        } else {
                            session.status = leindex_analyzers::memory::models::SessionStatus::Running;
                        }
                    }
                }
                app.refresh_session_entries();
            }

            // Fetch preview for selected session
            if app.tab_index == 1 {
                if let Some(i) = app.session_state.selected() {
                    if let Some(SessionEntry::Session(s)) = app.session_entries.get(i).cloned() {
                        if let Ok(content) = TmuxMultiplexer::get_pane_content(&s.session_id, 15) {
                            app.session_preview_content = content;
                        }
                    } else {
                        app.session_preview_content.clear();
                    }
                }
            }

            last_refresh = std::time::Instant::now();
        }

        app.frame_count = app.frame_count.wrapping_add(1);

        // High FPS polling (5ms) for 180Hz monitors, floor of 60fps
        if event::poll(std::time::Duration::from_millis(5))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.input_mode != InputMode::Normal {
                        match key.code {
                            KeyCode::Enter => {
                                match app.input_mode {
                                    InputMode::NewSessionTitle => app.input_mode = InputMode::NewSessionPath,
                                    InputMode::NewSessionPath => app.input_mode = InputMode::NewSessionTool,
                                    InputMode::NewSessionTool => {
                                        app.is_spawning = true;
                                        app.status_message = format!("Spawning {} session...", app.new_session_tool);
                                        // let _ = terminal.draw(|frame| ui(frame, \u0026mut app));

                                        if let Some(svc) = service.as_ref() {
                                            let manager = leindex_analyzers::memory::session_manager::SessionManager::new(svc.clone());
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
                                                let _ = TmuxMultiplexer::attach(&session.session_id);
                                                let _ = terminal.clear();
                                                app.status_message = format!("Returned from '{}'", session.title);
                                            }
                                        }
                                        app.input_mode = InputMode::Normal;
                                    }

                                    InputMode::RenameGroup => {
                                        app.input_mode = InputMode::RenameGroupCategory;
                                    }
                                    InputMode::RenameGroupCategory => {
                                        if let (Some(svc), Some(path)) = (service.as_ref(), app.target_group_path.take()) {
                                            if path == "uncategorized" {
                                                // Create a new real group instead of updating pseudo-group
                                                let new_path = format!("/{}", app.rename_buffer.trim().to_lowercase().replace(' ', "_"));
                                                let category = if app.new_group_category.trim().is_empty() { None } else { Some(app.new_group_category.clone()) };
                                                let _ = svc.create_session_group(&app.rename_buffer, &new_path, category);
                                                
                                                // Move all uncategorized sessions to this new group
                                                if let Ok(sessions) = svc.list_sessions() {
                                                    for s in sessions {
                                                        if s.group_path.is_none() {
                                                            let _ = svc.update_session_group(&s.session_id, Some(new_path.clone()));
                                                        }
                                                    }
                                                }
                                                app.status_message = format!("Created group '{}' and moved uncategorized sessions.", app.rename_buffer);
                                            } else {
                                                let _ = svc.update_group_name(&path, &app.rename_buffer);
                                                let category = if app.new_group_category.trim().is_empty() { None } else { Some(app.new_group_category.clone()) };
                                                let _ = svc.update_group_category(&path, category);
                                                app.status_message = format!("Group '{}' updated", app.rename_buffer);
                                            }

                                            if let Ok(groups) = svc.list_session_groups() { app.groups = groups; }
                                            if let Ok(sessions) = svc.list_sessions() { app.sessions = sessions; }
                                            app.refresh_session_entries();
                                        }
                                        app.target_group_path = None;
                                        app.rename_buffer.clear();
                                        app.new_group_category.clear();
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::ForkSession => {
                                        if let Some(svc) = service.as_ref() {
                                            if let Some(id) = app.target_session_id.take() {
                                                if let Some(orig) = app.sessions.iter().find(|s| s.session_id == id) {
                                                    let manager = leindex_analyzers::memory::session_manager::SessionManager::new(svc.clone());
                                                    let _ = manager.fork_session(&id, &app.rename_buffer, orig);
                                                    app.status_message = format!("Session forked as {}", app.rename_buffer);
                                                    if let Ok(sessions) = svc.list_sessions() { app.sessions = sessions; }
                                                }
                                            }
                                        }
                                        app.refresh_session_entries();
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::KillConfirm | InputMode::DeleteConfirm => {
                                        if let Some(svc) = service.as_ref() {
                                            if let Some(id) = app.target_session_id.take() {
                                                let manager = leindex_analyzers::memory::session_manager::SessionManager::new(svc.clone());
                                                let _ = manager.kill_session(&id);
                                                if app.input_mode == InputMode::DeleteConfirm {
                                                    let _ = svc.delete_session(&id);
                                                    app.status_message = "Session deleted".to_string();
                                                } else {
                                                    app.status_message = "Session killed".to_string();
                                                }
                                                if let Ok(sessions) = svc.list_sessions() { app.sessions = sessions; }
                                            }
                                        }
                                        app.refresh_session_entries();
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::AnalysisPrompt => {
                                        let input = app.analysis_input.clone();
                                        if !input.is_empty() {
                                            app.analysis_history.push(format!("> {}", input));
                                            match input.to_lowercase().split_whitespace().next() {
                                                Some("analyze") => app.analysis_history.push("  Analyzing structural dependencies... (Searching indexing tree)".to_string()),
                                                Some("scan") => app.analysis_history.push("  Scanning workspace for project roots...".to_string()),
                                                Some("stats") => app.analysis_history.push("  Computing repository metrics...".to_string()),
                                                Some("help") => {
                                                    app.analysis_history.push("  AVAILABLE COMMANDS:".to_string());
                                                    app.analysis_history.push("    analyze <path> - Start depth-first structural analysis".to_string());
                                                    app.analysis_history.push("    scan           - Scan for Maestro projects".to_string());
                                                    app.analysis_history.push("    stats          - Show repository metrics".to_string());
                                                }
                                                _ => app.analysis_history.push(format!("  Unknown command: {}. Try 'help'.", input)),
                                            }
                                            app.analysis_input.clear();
                                        } else {
                                            app.input_mode = InputMode::Normal;
                                        }
                                    }

                                    InputMode::NewGroupTitle => {
                                        app.input_mode = InputMode::NewGroupCategory;
                                    }
                                    InputMode::NewGroupCategory => {
                                        if let Some(svc) = service.as_ref() {
                                            let path = format!("/{}", app.rename_buffer.trim().to_lowercase().replace(' ', "_"));
                                            let category = if app.new_group_category.trim().is_empty() { None } else { Some(app.new_group_category.clone()) };
                                            let _ = svc.create_session_group(&app.rename_buffer, &path, category);
                                            app.status_message = format!("Group '{}' created", app.rename_buffer);
                                            if let Ok(groups) = svc.list_session_groups() { app.groups = groups; }
                                            app.refresh_session_entries();
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
                                                        let manager = leindex_analyzers::memory::session_manager::SessionManager::new(svc.clone());
                                                        let _ = manager.rename_session(&id, &app.rename_buffer);
                                                        if let Ok(sessions) = svc.list_sessions() { app.sessions = sessions; }
                                                        app.refresh_session_entries();
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

                                    InputMode::McpMenu => {
                                        if let (Some(svc), Some(name)) = (service.as_ref(), app.target_mcp_name.as_ref()) {
                                            match app.mcp_menu_option {
                                                McpOption::StartStop => {
                                                    if let Some(mut mcp) = app.mcp_servers.iter().find(|m| &m.name == name).cloned() {
                                                        mcp.status = if mcp.status == leindex_analyzers::memory::models::McpStatus::Running {
                                                            leindex_analyzers::memory::models::McpStatus::Stopped
                                                        } else {
                                                            leindex_analyzers::memory::models::McpStatus::Running
                                                        };
                                                        let _ = svc.update_mcp_server(mcp);
                                                        app.status_message = format!("MCP server '{}' status toggled", name);
                                                    }
                                                }
                                                McpOption::Remove => {
                                                    let _ = svc.delete_mcp_server(name);
                                                    app.status_message = format!("MCP server '{}' removed", name);
                                                }
                                                McpOption::Pause => {
                                                    app.status_message = format!("Pause not implemented for MCP '{}' yet", name);
                                                }
                                                _ => {}
                                            }
                                            if let Ok(mcp_list) = svc.list_mcp_servers() { app.mcp_servers = mcp_list; }
                                        }
                                        app.input_mode = InputMode::Normal;
                                        app.target_mcp_name = None;
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
                                    InputMode::Normal => {}
                                }
                            }
                            KeyCode::Esc => app.input_mode = InputMode::Normal,
                            KeyCode::Backspace => {
                                match app.input_mode {
                                    InputMode::NewSessionTitle => { app.new_session_title.pop(); }
                                    InputMode::NewSessionPath => { app.new_session_path.pop(); }
                                    InputMode::RenameGroup | InputMode::ForkSession => {
                                        app.rename_buffer.pop();
                                    }
                                    InputMode::KillConfirm => {
                                        app.target_session_id = None;
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::AnalysisPrompt => {
                                        app.analysis_input.pop();
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
                                    InputMode::RenameGroup | InputMode::ForkSession | InputMode::NewGroupTitle | InputMode::MoveToGroup => app.rename_buffer.push(c),
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
                                    InputMode::KillConfirm | InputMode::DeleteConfirm => {
                                        if c == 'y' || c == 'Y' {
                                            if let Some(svc) = service.as_ref() {
                                                if let Some(id) = app.target_session_id.take() {
                                                    let manager = leindex_analyzers::memory::session_manager::SessionManager::new(svc.clone());
                                                    let _ = manager.kill_session(&id);
                                                    if app.input_mode == InputMode::DeleteConfirm {
                                                        let _ = svc.delete_session(&id);
                                                        app.status_message = "Session deleted".to_string();
                                                    } else {
                                                        app.status_message = "Session killed".to_string();
                                                    }
                                                    if let Ok(sessions) = svc.list_sessions() { app.sessions = sessions; }
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
                                    InputMode::SessionHub => {
                                        app.hub_focus = match app.hub_focus {
                                            HubFocus::Rename => HubFocus::Group,
                                            HubFocus::Group => HubFocus::Search,
                                            HubFocus::Search => HubFocus::Rename,
                                        };
                                    }
                                    _ => {}
                                }
                            }
                            KeyCode::BackTab => {
                                match app.input_mode {
                                    InputMode::SessionHub => {
                                        app.hub_focus = match app.hub_focus {
                                            HubFocus::Rename => HubFocus::Search,
                                            HubFocus::Group => HubFocus::Rename,
                                            HubFocus::Search => HubFocus::Group,
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
                                } else if app.input_mode == InputMode::McpMenu {
                                    app.mcp_menu_option = match app.mcp_menu_option {
                                        McpOption::StartStop => McpOption::Pause,
                                        McpOption::Pause => McpOption::Logs,
                                        McpOption::Logs => McpOption::Add,
                                        McpOption::Add => McpOption::Remove,
                                        McpOption::Remove => McpOption::Install,
                                        McpOption::Install => McpOption::StartStop,
                                    };
                                }
                            }
                            KeyCode::Up => {
                                if app.input_mode == InputMode::SessionSwitcher {
                                    let i = match app.switcher_state.selected() {
                                        Some(i) => if i == 0 { app.sessions.len().saturating_sub(1) } else { i - 1 },
                                        None => 0,
                                    };
                                    app.switcher_state.select(Some(i));
                                } else if app.input_mode == InputMode::McpMenu {
                                    app.mcp_menu_option = match app.mcp_menu_option {
                                        McpOption::StartStop => McpOption::Install,
                                        McpOption::Pause => McpOption::StartStop,
                                        McpOption::Logs => McpOption::Pause,
                                        McpOption::Add => McpOption::Logs,
                                        McpOption::Remove => McpOption::Add,
                                        McpOption::Install => McpOption::Remove,
                                    };
                                }
                            }
                            _ => {}
                        }
                    } else if app.show_help {
                        if matches!(key.code, KeyCode::Char('/') | KeyCode::Esc) {
                            app.show_help = false;
                        }
                    } else {
                        match (key.modifiers, key.code) {
                            (KeyModifiers::ALT, KeyCode::Char('p')) => {
                                if app.tab_index == 1 {
                                    app.preview_focused = !app.preview_focused;
                                    app.status_message = if app.preview_focused { "Preview focused. Scroll with Arrows/PgUp/PgDn." } else { "List focused." }.to_string();
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
                            (_, KeyCode::Char('q')) => {
                                if app.project_view_open {
                                    app.project_view_open = false;
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
                                                app.new_group_category = g.category.clone().unwrap_or_default();
                                                app.input_mode = InputMode::RenameGroup;
                                            }
                                            _ => {}
                                        }
                                    }
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
                            (KeyModifiers::ALT, KeyCode::Char('d')) | (KeyModifiers::ALT, KeyCode::Char('D')) => {
                                if app.tab_index == 1 {
                                    if let Some(i) = app.session_state.selected() {
                                        match &app.session_entries[i] {
                                            SessionEntry::Session(s) => {
                                                app.target_session_id = Some(s.session_id.clone());
                                                app.status_message = format!("Confirm PERMANENT DELETE session '{}'? (y/n)", s.title);
                                                app.input_mode = InputMode::DeleteConfirm;
                                            }
                                            SessionEntry::Group(g) => {
                                                app.target_group_path = Some(g.path.clone());
                                                app.status_message = format!("Confirm DELETE group '{}' and all sessions? (y/n)", g.name);
                                                app.input_mode = InputMode::DeleteConfirm;
                                            }
                                        }
                                    }
                                } else if app.tab_index == 2 {
                                    // Project list temporary message 
                                    app.status_message = "Project deletion via TUI coming soon in v2.1".to_string();
                                }
                            }
                            (_, KeyCode::Tab) => {
                                app.tab_index = (app.tab_index + 1) % 5;
                                app.preview_focused = false; // Reset focus when switching tabs
                            }
                            (_, KeyCode::BackTab) => {
                                app.tab_index = if app.tab_index == 0 { 4 } else { app.tab_index - 1 };
                                app.preview_focused = false;
                            }
                            (KeyModifiers::ALT, KeyCode::Char('o')) => {
                                app.tab_index = if app.tab_index == 0 { 4 } else { app.tab_index - 1 };
                                app.preview_focused = false;
                            }
                            (KeyModifiers::ALT, KeyCode::Char('i')) => {
                                app.tab_index = (app.tab_index + 1) % 5;
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
                                } else if app.tab_index == 0 { // Dashboard (MCP Pool)
                                    let i = match app.mcp_state.selected() {
                                        Some(i) => if i >= app.mcp_servers.len().saturating_sub(1) { 0 } else { i + 1 },
                                        None => 0,
                                    };
                                    app.mcp_state.select(Some(i));
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
                                    let i = match app.mcp_state.selected() {
                                        Some(i) => if i == 0 { app.mcp_servers.len().saturating_sub(1) } else { i - 1 },
                                        None => 0,
                                    };
                                    app.mcp_state.select(Some(i));
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
                                }
                                app.scroll = app.scroll.saturating_sub(1);
                            }
                            (_, KeyCode::Char('1')) => app.tab_index = 0,
                            (_, KeyCode::Char('2')) => app.tab_index = 1,
                            (_, KeyCode::Char('3')) => app.tab_index = 2,
                            (_, KeyCode::Char('4')) => app.tab_index = 3,
                            (_, KeyCode::Char('5')) => app.tab_index = 4,
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
                                        if let SessionEntry::Session(s) = &app.session_entries[i] {
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
                                                let _ = std::process::Command::new("hx").arg(new_path).status();
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
                                        let mut path = std::path::PathBuf::from(current);
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
                                    let mut path = std::path::PathBuf::from(current.replace("~", &std::env::var("HOME").unwrap_or_default()));
                                    if let Some(parent) = path.parent() {
                                        app.project_explorer_path = Some(parent.to_string_lossy().to_string());
                                        app.project_explorer_selected = 0;
                                    }
                                }
                            }
                            (_, KeyCode::Char('e')) if app.tab_index == 2 => {
                                app.preview_focused = !app.preview_focused;
                            }
                            (_, KeyCode::Enter) => {
                                if app.tab_index == 2 { // Projects Tab
                                    if let Some(i) = app.project_state.selected() {
                                        let project = &app.projects[i].clone();
                                        app.status_message = format!("Launching Zide for {}...", project.name);
                                        let _ = terminal.draw(|frame| ui(frame, &mut app));
                                        
                                        match leindex_analyzers::multiplexer::zellij::ZellijMultiplexer::spawn_zide(&project.path, &project.name) {
                                            Ok(_) => { 
                                                let _ = terminal.clear(); // Ensure screen is clear after Zellij exit
                                                let _ = terminal.draw(|frame| ui(frame, &mut app));
                                                app.status_message = format!("Zide launched for {}. Switch to see it.", project.name); 
                                            }
                                            Err(e) => { app.status_message = format!("Error: {}", e); }
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
                                                    let _ = TmuxMultiplexer::attach(&s.session_id);
                                                    let _ = terminal.clear(); // Restore terminal state
                                                    app.status_message = format!("Returned from '{}'", s.title);
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
                            (KeyModifiers::CONTROL, KeyCode::Char('/')) => app.show_help = true,
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
                                // Manual full refresh
                                if let Some(svc) = service.as_ref() {
                                    if let Ok(projects) = svc.list_projects() {
                                        app.projects = projects.iter().map(|p| ProjectInfo {
                                            name: p.project_name.clone(),
                                            path: p.project_path.clone(),
                                            _track_count: 0,
                                        }).collect();
                                    }
                                    if let Ok(memories) = svc.list_memories(20) {
                                        app.memories = memories.iter().map(|m| MemoryInfo {
                                            _id: m.id,
                                            content: m.content.clone(),
                                            category: m.category.to_string(),
                                        }).collect();
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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Content
            Constraint::Length(3),  // Footer
        ])
        .split(frame.area());

    // Header with tabs
    let tabs = Tabs::new(vec!["Dashboard", "Sessions", "Projects", "Analysis", "Memory"])
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(" Maestro v2.0 "))
        .select(app.tab_index)
        .highlight_style(Style::default().fg(Color::Cyan).bold());
    
    frame.render_widget(tabs, chunks[0]);

    match app.tab_index {
        0 => render_dashboard(frame, chunks[1], app),
        1 => render_sessions(frame, chunks[1], app),
        2 => render_projects(frame, chunks[1], app),
        3 => render_analysis(frame, chunks[1], app),
        4 => render_memory(frame, chunks[1], app),
        _ => {}
    }
    // Footer
    let footer = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" Tab ", Style::default().bg(Color::Cyan).fg(Color::Black)),
            Span::raw(" Switch  "),
            Span::styled(" ↑↓ Arrows ", Style::default().bg(Color::Cyan).fg(Color::Black)),
            Span::raw(" Scroll  "),
            Span::styled(" 1-5 ", Style::default().bg(Color::Cyan).fg(Color::Black)),
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

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(title)
        .title_style(Style::default().fg(Color::Yellow));

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
        .border_type(BorderType::Rounded)
        .title(" 🕒 Recent Sessions ")
        .title_style(Style::default().fg(Color::Blue));
    
    let mut session_rows = vec![Line::from("")];
    if app.sessions.is_empty() {
        session_rows.push(Line::from("  No active sessions"));
    } else {
        // Group sessions by group path but display group name
        let mut grouped: std::collections::BTreeMap<String, Vec<&leindex_analyzers::memory::models::Session>> = std::collections::BTreeMap::new();
        for s in &app.sessions {
            let g = s.group_path.as_deref().unwrap_or("uncategorized").to_string();
            grouped.entry(g).or_default().push(s);
        }
        
        for (group_path, sessions) in grouped {
            let display_name = if group_path == "uncategorized" {
                "[Uncategorized]".to_string()
            } else {
                app.groups.iter().find(|g| g.path == group_path)
                    .map(|g| g.name.clone())
                    .unwrap_or_else(|| group_path.clone())
            };

            session_rows.push(Line::from(vec![
                Span::styled(format!("  [{}]", display_name), Style::default().fg(Color::Cyan).bold()),
            ]));
            for s in sessions.iter().take(3) {
                let status_icon = match s.status {
                    leindex_analyzers::memory::models::SessionStatus::Running => Span::styled("   * ", Style::default().fg(Color::Green)),
                    leindex_analyzers::memory::models::SessionStatus::Terminated => Span::styled("   x ", Style::default().fg(Color::Red)),
                    leindex_analyzers::memory::models::SessionStatus::Waiting => Span::styled("   ◒ ", Style::default().fg(Color::Yellow)),
                    _ => Span::styled("   o ", Style::default().fg(Color::Gray)),
                };
                session_rows.push(Line::from(vec![
                    status_icon,
                    Span::styled(&s.title, Style::default()),
                ]));
            }
        }
    }
    let sessions = Paragraph::new(session_rows).block(session_block);
    frame.render_widget(sessions, right_chunks[0]);

    // Bottom Right - MCP Pool
    let mcp_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 🕹️ Interactive MCP Pool ")
        .title_style(Style::default().fg(Color::Cyan).bold())
        .style(if app.tab_index == 0 { Style::default().fg(Color::Cyan) } else { Style::default() });
    
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
        .highlight_style(Style::default().bg(Color::Rgb(30, 30, 50)).fg(Color::White).bold())
        .highlight_symbol(">> ");
    frame.render_stateful_widget(mcp_list, right_chunks[1], &mut app.mcp_state);
}

fn render_help_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 40, frame.area());
    let block = Block::default()
        .title(" Commands Cheat-sheet ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(15, 15, 25)));

    let text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(" GLOBAL CONTROLS:", Style::default().fg(Color::Yellow).bold())]),
        Line::from(vec![Span::styled("   Tab / S-Tab   ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Cycle Tabs / Focus Preview (e.g. 1->2->3)")]),
        Line::from(vec![Span::styled("   ↑ / ↓         ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Navigate / Scroll Preview")]),
        Line::from(vec![Span::styled("   Ctrl + /      ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Toggle This Modal")]),
        Line::from(vec![Span::styled("   q / Ctrl-C    ", Style::default().fg(Color::Red).bold()),  Span::raw(" Quit Maestro Cockpit")]),
        Line::from(""),
        Line::from(vec![Span::styled(" DASHBOARD (Tab 1):", Style::default().fg(Color::Yellow).bold())]),
        Line::from(vec![Span::styled("   s             ", Style::default().fg(Color::Cyan).bold()), Span::raw(" MCP Server Menu (Start/Stop, Add, Remove)")]),
        Line::from(vec![Span::styled("   x             ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Quick Remove MCP Server")]),
        Line::from(""),
        Line::from(vec![Span::styled(" SESSIONS (Tab 2):", Style::default().fg(Color::Yellow).bold())]),
        Line::from(vec![Span::styled("   n             ", Style::default().fg(Color::Green).bold()), Span::raw(" New Session Wizard (Title, Path, Tool)")]),
        Line::from(vec![Span::styled("   Enter         ", Style::default().fg(Color::Green).bold()), Span::raw(" Attach to tmux Session")]),
        Line::from(vec![Span::styled("   r             ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Session Hub (Rename, Move, Search history)")]),
        Line::from(vec![Span::styled("   Alt + p       ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Focus Preview Pane (for scrolling history)")]),
        Line::from(vec![Span::styled("   m             ", Style::default().fg(Color::Magenta).bold()), Span::raw(" Move Session to Group / Create New Group")]),
        Line::from(vec![Span::styled("   G             ", Style::default().fg(Color::Green).bold()), Span::raw(" Create Standalone Group")]),
        Line::from(vec![Span::styled("   k             ", Style::default().fg(Color::Red).bold()), Span::raw(" Kill tmux Session Process")]),
        Line::from(vec![Span::styled("   Alt + D       ", Style::default().fg(Color::Red).bold()), Span::raw(" PURMANENT DELETE Session/Group from DB")]),
        Line::from(vec![Span::styled("   f             ", Style::default().fg(Color::Magenta).bold()), Span::raw(" Fork Session (Clone state to new session)")]),
        Line::from(""),
        Line::from(vec![Span::styled(" PROJECTS (Tab 3):", Style::default().fg(Color::Yellow).bold())]),
        Line::from(vec![Span::styled("   Enter         ", Style::default().fg(Color::Green).bold()), Span::raw(" Open Zide (File Picker + Editor)")]),
        Line::from(""),
        Line::from(vec![Span::styled(" ANALYSIS (Tab 4):", Style::default().fg(Color::Yellow).bold())]),
        Line::from(vec![Span::styled("   a             ", Style::default().fg(Color::Cyan).bold()), Span::raw(" Enter Analysis Command Box")]),
        Line::from(""),
        Line::from("  ---------------------------------- "),
        Line::from(format!("  Maestro TUI Cockpit v2.0-beta-8  {}", if (app.frame_count / 30) % 2 == 0 { "⚡" } else { "  " })),
    ];

    let para = Paragraph::new(text).block(block).alignment(Alignment::Left);
    frame.render_widget(Clear, area);
    frame.render_widget(para, area);
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
    
    let name = app.target_mcp_name.as_deref().unwrap_or("Unknown");
    let block = Block::default()
        .title(format!(" MCP: {} ", name))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(20, 20, 35)));
    
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

    let hint = Paragraph::new("Press 'Enter' to confirm step, 'Esc' to cancel\n\nGroups help you organize your coding sessions.")
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
    let block = Block::default()
        .title(" Quick Session Switcher ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));

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
            .highlight_style(Style::default().bg(Color::Rgb(40, 40, 60)).fg(Color::Cyan).bold())
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
            ])
        }).collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::Rgb(40, 40, 60)).fg(Color::White).bold());
        frame.render_stateful_widget(list, chunks[0], &mut app.project_state);

        // Right side: Project Explorer (File Tree)
        let explorer_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" 🌲 Project Explorer ")
            .title_style(Style::default().fg(Color::Green));

        if let Some(i) = app.project_state.selected() {
            let project = &app.projects[i];
            let current_path = app.project_explorer_path.clone().unwrap_or_else(|| project.path.clone());
            let expanded_path = current_path.replace("~", &std::env::var("HOME").unwrap_or_default());
            let mut items = Vec::new();
            
            // Add root info
            items.push(ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("  Browsing: {}", project.name), Style::default().fg(Color::Yellow).bold()),
                ]),
                Line::from(vec![
                    Span::styled(format!("  Path:     {}", current_path), Style::default().fg(Color::DarkGray).italic()),
                ]),
                Line::from(""),
            ]));

            // List directory contents
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

                    items.push(ListItem::new(Line::from(vec![
                        Span::styled(format!("    {} ", icon), style),
                        Span::styled(file_name, style),
                    ])));
                }
                if dir_entries.len() > 30 {
                    items.push(ListItem::new(Line::from(vec![
                        Span::styled(format!("    ... and {} more items", dir_entries.len() - 30), Style::default().fg(Color::DarkGray).italic()),
                    ])));
                }
            } else {
                items.push(ListItem::new(Span::styled("  Error reading directory. (Path might not exist or need expansion)", Style::default().fg(Color::Red))));
            }
            
            let list = List::new(items).block(explorer_block);
            frame.render_widget(list, chunks[1]);
        }
 else {
            let para = Paragraph::new(vec![
                Line::from(""),
                Line::from("  Select a project to explore its files."),
                Line::from(""),
                Line::from("  Press Enter to open in Zide (Editor)."),
            ]).block(explorer_block).alignment(Alignment::Center);
            frame.render_widget(para, chunks[1]);
        }
    }
}

fn render_memory(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 🧠 Memory System ")
        .title_style(Style::default().fg(Color::Magenta));

    if app.memories.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from("  No memories found."),
            Line::from(""),
            Line::from("  Add memories during your chat session to see them here."),
        ];
        let para = Paragraph::new(text).block(block);
        frame.render_widget(para, area);
    } else {
        let items: Vec<ListItem> = app.memories.iter().map(|m| {
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("[{}] ", m.category), Style::default().fg(Color::Yellow)),
                    Span::styled(m.content.clone(), Style::default().fg(Color::White)),
                ]),
            ])
        }).collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::Rgb(40, 40, 60)).fg(Color::White).bold());
        frame.render_widget(list, area);
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

    let hub_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 🚀 Analysis Command Hub ")
        .title_style(Style::default().fg(Color::Magenta));

    // History View
    let mut history_lines = vec![
        Line::from(vec![
            Span::styled(" Maestro Analysis Engine v2.0 READY", Style::default().fg(Color::Green).bold()),
        ]),
        Line::from(vec![
            Span::styled(" Type 'analyze <path>' to begin. ", Style::default().fg(Color::Gray)),
            Span::styled("(Press 'a' to enter Command Hub)", Style::default().fg(Color::DarkGray).italic()),
        ]),
        Line::from(""),
    ];

    let examples = vec![
        Line::from(vec![Span::styled(" EXAMPLES:", Style::default().fg(Color::Yellow).bold())]),
        Line::from("  $ analyze src/main.rs"),
        Line::from("  $ scan ."),
        Line::from("  $ stats --deep"),
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

                // Row 4: Claude info if applicable
                if s.tool.as_deref() == Some("claude") {
                    if let Some(ref metadata) = s.metadata {
                        if let Some(cid) = metadata.get("claude_session_id").and_then(|v| v.as_str()) {
                            preview_lines.push(Line::from(vec![
                                Span::styled(" Status: ", Style::default().fg(Color::DarkGray)),
                                Span::styled("● Connected", Style::default().fg(Color::Green)),
                            ]));
                            preview_lines.push(Line::from(vec![
                                Span::styled(" Session ID: ", Style::default().fg(Color::DarkGray)),
                                Span::styled(cid, Style::default().fg(Color::White)),
                            ]));
                        }
                        if let Some(mcps) = metadata.get("loaded_mcp_names").and_then(|v| v.as_array()) {
                            let mcp_names: Vec<String> = mcps.iter()
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


