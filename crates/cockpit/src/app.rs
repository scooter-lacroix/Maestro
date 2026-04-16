//! TUI command implementation
//!
//! Beautiful Terminal User Interface using ratatui.
//! Shows projects, memories, and analysis status.

use anyhow::{Context, Result};
use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear as TerminalClear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Tabs, Wrap},
};
use std::hash::{Hash, Hasher};
use std::{
    collections::hash_map::DefaultHasher,
    collections::{BTreeMap, HashMap, HashSet},
    fs, io,
    io::Write,
    path::PathBuf,
    sync::Arc,
    time::Instant,
};
use tracing::debug;

use leindex_core::config::Config;
use leindex_core::memory::lsp_manager::LspType;
use leindex_core::memory::mcp_installer::ManagedMcpInstaller;
use leindex_core::memory::models::McpManagedInstallManifest;
use leindex_core::memory::turso_backend::LspStatus;
use leindex_core::memory::turso_backend::TursoStorageBackend;
use leindex_core::memory::McpPool;
use leindex_core::memory::MemoryService;
use leindex_core::multiplexer::TmuxMultiplexer;
use leindex_core::provider_boundary::{
    managed_cli_overlap_matrix_for, managed_cli_overlap_profile, RuntimeDiagnostics,
};

// Phase 3: Capabilities
use maestro_core::{CronJob, McpManager, SandboxManager, SecurityPolicy};

use crate::conductor::omp_agent::OmpAgentManager;
use crate::maesterclaw::claw_loop::ClawLoop;
use crate::maesterclaw::pty_bridge::PtyLaunchConfig;
use crate::maesterclaw::{
    AgentOutputLine, ClawSession, ClawSessionStatus, ClawViewMode, OutputLineType,
};
use crate::modals;
use crate::omp::{is_omp_available, OmpWorkerStatus};
use crate::state::{
    AnalysisMode, DashFocus, DashSessionEntry, HubFocus, InputMode, McpOption, MemoryInfo,
    ProjectInfo, SessionEntry, SettingsMenuKind, SettingsOption, Stats,
};
use crate::tabs::render_tracklens;
use crate::tabs::settings::render_settings;
use crate::tracklens::TrackLensPane;

// Re-export for use in tabs
pub use crate::tabs::ktop::{render_ktop, KtopState};
use crate::theme::{theme_from_name, Theme, THEMES};

const MAX_MAESTROCLAW_OUTPUT_LINES: usize = 2000;

/// Tab identifiers with explicit indices for maintainability
/// Order: Welcome(0) → MaestroClaw(1) → Sessions(2) → Projects(3) → Conductor(4) → Memory(5) → Analysis(6) → Krustop(7) → LSPs(8) → Settings(9) → TrackLens(10)
pub mod tabs {
    pub const DASHBOARD: usize = 0;
    pub const MAESTROCLAW: usize = 1;
    pub const SESSIONS: usize = 2;
    pub const PROJECTS: usize = 3;
    pub const CONDUCTOR: usize = 4;
    pub const MEMORY: usize = 5;
    pub const ANALYSIS: usize = 6;
    pub const KRUSTOP: usize = 7;
    pub const LSPS: usize = 8;
    pub const SETTINGS: usize = 9;
    pub const TRACKLENS: usize = 10;

    /// Get all tab titles in order
    pub fn all_titles() -> Vec<&'static str> {
        vec![
            "Welcome",
            "MaestroClaw",
            "Sessions",
            "Projects",
            "Conductor",
            "Memory",
            "Analysis",
            "Krustop",
            "LSPs",
            "Settings",
            "TrackLens",
        ]
    }
}

pub async fn run() -> Result<()> {
    // Initialize file logging (before anything else so all operations are captured)
    // Non-fatal: if logging can't start (disk full, unwritable path), continue without it
    let _log_guard = match crate::cockpit_log::init() {
        Ok(guard) => Some(guard),
        Err(err) => {
            eprintln!("Warning: failed to initialize cockpit logging: {err}");
            None
        }
    };

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
        let _ = s.sync_projects_from_system(); // Auto-discover and register Maestro projects
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
    pub memory_graph_enabled: bool,
    pub memory_graph_selection: usize,
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
    // Phase 3: Capabilities tab state
    pub capabilities_section: Option<crate::tabs::CapabilitiesSection>,
    // Phase 3: Capabilities services
    pub cron_jobs: Vec<CronJob>,
    pub cron_job_state: ratatui::widgets::ListState,
    pub mcp_manager: maestro_core::McpManager,
    pub sandbox_manager: maestro_core::SandboxManager,
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
    // Phase 7.7: Hot cache for memory suggestions
    pub hot_cache: crate::maesterclaw::HotCache,
    // MaestroClaw pane state
    pub maestroclaw_pane: crate::maesterclaw::MaestroClawPane,
    pub maestroclaw_runtime: Option<ClawLoop>,
    // TrackLens review state
    pub tracklens_pane: TrackLensPane,
}

// Note: Type definitions (InputMode, HubFocus, McpOption, SettingsOption, SettingsMenuKind,
// DashFocus, SessionEntry, DashSessionEntry, ProjectInfo, MemoryInfo, Stats) are now
// imported from crate::state module to avoid duplication.

/// Load the maestro-claw config and extract the workspace directory for
/// `MaestroClawPane::new()`.  This is the single source of truth for the
/// config-to-pane wiring used by `App::new` so that a regression in the
/// startup path cannot slip through undetected.
fn load_maestroclaw_workspace_dir() -> PathBuf {
    maestro_claw::config::Config::load()
        .unwrap_or_else(|_| maestro_claw::config::Config::default())
        .workspace_dir
}

fn parse_memory_stored_by(source: Option<&str>, metadata: Option<&serde_json::Value>) -> String {
    let Some(metadata) = metadata else {
        return source
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("unknown")
            .to_string();
    };

    for key in [
        "stored_by",
        "created_by",
        "agent_id",
        "agent",
        "source_agent",
        "tool_used",
        "tool",
    ] {
        if let Some(value) = metadata.get(key).and_then(|value| value.as_str()) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    if let Some(source) = source {
        let trimmed = source.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    "unknown".to_string()
}

fn parse_memory_access_history(
    metadata: Option<&serde_json::Value>,
) -> Vec<crate::state::MemoryAccessEvent> {
    let Some(events) = metadata.and_then(|metadata| {
        metadata
            .get("access_history")
            .or_else(|| metadata.get("nexus_access_history"))
            .or_else(|| metadata.get("access_events"))
            .and_then(|value| value.as_array())
    }) else {
        return Vec::new();
    };

    events
        .iter()
        .filter_map(|event| {
            // Extract raw values first, before applying defaults
            let raw_agent_id = event
                .get("agent_id")
                .or_else(|| event.get("agent"))
                .and_then(|value| value.as_str())
                .map(|s| s.trim());
            let raw_timestamp = event
                .get("timestamp")
                .or_else(|| event.get("at"))
                .and_then(|value| value.as_str())
                .map(|s| s.trim());
            let raw_tool_used = event
                .get("tool_used")
                .or_else(|| event.get("tool"))
                .and_then(|value| value.as_str())
                .map(|s| s.trim());

            // Skip if all raw values are empty or None
            let all_empty = raw_agent_id.map_or(true, |s| s.is_empty())
                && raw_timestamp.map_or(true, |s| s.is_empty())
                && raw_tool_used.map_or(true, |s| s.is_empty());
            if all_empty {
                return None;
            }

            // Now apply defaults for the struct
            let agent_id = raw_agent_id
                .filter(|v| !v.is_empty())
                .unwrap_or("unknown")
                .to_string();
            let timestamp = raw_timestamp.unwrap_or("").to_string();
            let access_type = event
                .get("access_type")
                .or_else(|| event.get("type"))
                .and_then(|value| value.as_str())
                .unwrap_or("read")
                .trim()
                .to_string();
            let tool_used = raw_tool_used.map(|s| s.to_string()).filter(|s| !s.is_empty());

            Some(crate::state::MemoryAccessEvent {
                agent_id,
                timestamp,
                tool_used,
                access_type,
            })
        })
        .collect()
}

fn parse_memory_related_ids(metadata: Option<&serde_json::Value>) -> Vec<i64> {
    let Some(metadata) = metadata else {
        return Vec::new();
    };

    let mut ids = Vec::new();
    for key in [
        "related_memory_ids",
        "related_memories",
        "semantic_neighbors",
        "lineage",
        "nexus_related_ids",
    ] {
        if let Some(value) = metadata.get(key) {
            match value {
                serde_json::Value::Array(items) => {
                    for item in items {
                        if let Some(id) = item.as_i64() {
                            ids.push(id);
                            continue;
                        }
                        if let Some(id) = item
                            .get("id")
                            .or_else(|| item.get("memory_id"))
                            .and_then(|value| value.as_i64())
                        {
                            ids.push(id);
                        }
                    }
                }
                serde_json::Value::Number(number) => {
                    if let Some(id) = number.as_i64() {
                        ids.push(id);
                    }
                }
                _ => {}
            }
        }
    }

    ids.sort_unstable();
    ids.dedup();
    ids
}

fn parse_memory_runtime_state(metadata: Option<&serde_json::Value>) -> Option<String> {
    let metadata = metadata?;
    for key in [
        "nexus_runtime_state",
        "runtime_state",
        "subconscious_state",
        "memory_runtime_state",
    ] {
        if let Some(value) = metadata.get(key).and_then(|value| value.as_str()) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn parse_memory_scope(metadata: Option<&serde_json::Value>) -> Option<String> {
    let metadata = metadata?;
    for key in ["nexus_scope", "scope", "namespace", "memory_scope"] {
        if let Some(value) = metadata.get(key).and_then(|value| value.as_str()) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn parse_memory_accessed_by(
    metadata: Option<&serde_json::Value>,
    access_history: &[crate::state::MemoryAccessEvent],
) -> Vec<String> {
    let mut agents: Vec<String> = metadata
        .and_then(|metadata| metadata.get("accessed_by"))
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(|value| value.trim().to_string()))
                .filter(|value| !value.is_empty())
                .collect()
        })
        .unwrap_or_default();

    for event in access_history {
        if !event.agent_id.is_empty() && !agents.iter().any(|agent| agent == &event.agent_id) {
            agents.push(event.agent_id.clone());
        }
    }

    agents
}

fn parse_memory_access_count(
    metadata: Option<&serde_json::Value>,
    access_history: &[crate::state::MemoryAccessEvent],
) -> usize {
    metadata
        .and_then(|metadata| metadata.get("access_count"))
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .unwrap_or(access_history.len())
}

fn parse_memory_similarity_score(metadata: Option<&serde_json::Value>) -> Option<f32> {
    metadata
        .and_then(|metadata| metadata.get("similarity_score"))
        .and_then(|value| value.as_f64())
        .map(|value| value as f32)
}

fn maestroclaw_shell_program() -> String {
    #[cfg(target_os = "windows")]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("SHELL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "bash".to_string())
    }
}

fn maestroclaw_shell_args(command: &str) -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        vec!["/C".to_string(), command.to_string()]
    }

    #[cfg(not(target_os = "windows"))]
    {
        vec!["-lc".to_string(), command.to_string()]
    }
}

fn fallback_maestroclaw_command(session: &leindex_core::memory::models::Session) -> String {
    match session
        .tool
        .as_deref()
        .unwrap_or("shell")
        .to_lowercase()
        .as_str()
    {
        "claude" => "claude".to_string(),
        "gemini" => "gemini".to_string(),
        "codex" => "codex".to_string(),
        "opencode" => "opencode".to_string(),
        "amp" => "amp".to_string(),
        "qwen" => "qwen".to_string(),
        "iflow" => "iflow".to_string(),
        "droid" => "droid".to_string(),
        _ => {
            // Use appropriate shell for the platform
            if cfg!(windows) {
                std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
            } else {
                std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string())
            }
        }
    }
}

fn build_maestroclaw_launch(
    session: &leindex_core::memory::models::Session,
) -> anyhow::Result<PtyLaunchConfig> {
    let selected_cli = session.tool.clone().unwrap_or_else(|| "shell".to_string());
    let overlap_profile = managed_cli_overlap_profile(&selected_cli);
    let overlap_matrix = managed_cli_overlap_matrix_for(&selected_cli);
    let suppression = overlap_profile.suppression_policy();
    let provider_profile = "maestro_runtime".to_string();
    let analysis_provider = "standalone_leindex".to_string();
    let memory_provider = "standalone_nexus".to_string();
    let suppression_policy =
        serde_json::to_string(&suppression).unwrap_or_else(|_| "{}".to_string());
    let cli_overlap_profile =
        serde_json::to_string(&overlap_profile).unwrap_or_else(|_| "{}".to_string());

    // Capture launch-time diagnostics from the overlap matrix
    let diagnostics = RuntimeDiagnostics {
        captured_at: chrono::Utc::now(),
        aggregate_status: leindex_core::provider_boundary::ProviderStatus::Healthy,
        suppressed_count: suppression.suppressed_tools.len(),
        analysis_preferred_count: suppression.analysis_preferred_tools.len(),
        memory_preferred_count: suppression.memory_preferred_tools.len(),
        retained_count: suppression.retained_maestro_tools.len(),
        overlap_entry_count: overlap_matrix.entries.len(),
        provider_details: vec![],
    };
    let diagnostics_summary = diagnostics.summary_line();

    let command = session
        .command
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback_maestroclaw_command(session));

    let cwd = if session.project_path.trim().is_empty() {
        None
    } else {
        Some(PathBuf::from(session.project_path.clone()))
    };

    Ok(PtyLaunchConfig {
        program: maestroclaw_shell_program(),
        args: maestroclaw_shell_args(&command),
        cwd,
        env: vec![
            ("MAESTRO_SESSION_ID".to_string(), session.session_id.clone()),
            (
                "MAESTRO_PROJECT_PATH".to_string(),
                session.project_path.clone(),
            ),
            ("MAESTRO_SELECTED_CLI".to_string(), selected_cli),
            ("MAESTRO_PROVIDER_PROFILE".to_string(), provider_profile),
            ("MAESTRO_ANALYSIS_PROVIDER".to_string(), analysis_provider),
            ("MAESTRO_MEMORY_PROVIDER".to_string(), memory_provider),
            (
                "MAESTRO_TOOL_SUPPRESSION_POLICY".to_string(),
                suppression_policy,
            ),
            (
                "MAESTRO_CLI_OVERLAP_PROFILE".to_string(),
                cli_overlap_profile,
            ),
            (
                "MAESTRO_NEXUS_PROVIDER".to_string(),
                "standalone".to_string(),
            ),
            (
                "MAESTRO_LAUNCH_DIAGNOSTICS".to_string(),
                diagnostics_summary,
            ),
            (
                "NEXUS_BIN".to_string(),
                std::env::var("NEXUS_BIN").unwrap_or_else(|_| "nexus".to_string()),
            ),
            (
                "NEXUS_HOME".to_string(),
                std::env::var("NEXUS_HOME").unwrap_or_default(),
            ),
        ],
        rows: 48,
        cols: 160,
    })
}

fn push_maestroclaw_lines(app: &mut App, lines: Vec<AgentOutputLine>) {
    if lines.is_empty() {
        return;
    }

    app.maestroclaw_pane.agent_output.extend(lines);
    if app.maestroclaw_pane.agent_output.len() > MAX_MAESTROCLAW_OUTPUT_LINES {
        let overflow = app.maestroclaw_pane.agent_output.len() - MAX_MAESTROCLAW_OUTPUT_LINES;
        app.maestroclaw_pane.agent_output.drain(0..overflow);
    }
}

fn sync_maestroclaw_runtime(app: &mut App) {
    let Some(runtime) = app.maestroclaw_runtime.as_mut() else {
        return;
    };

    let lines = runtime.poll();
    let session = runtime.session.clone();
    let _ = runtime;

    if !lines.is_empty() {
        push_maestroclaw_lines(app, lines);
    }
    app.maestroclaw_pane.claw_session = Some(session);
}

fn start_maestroclaw_session(
    app: &mut App,
    session: leindex_core::memory::models::Session,
) -> anyhow::Result<()> {
    if let Some(mut runtime) = app.maestroclaw_runtime.take() {
        let _ = runtime.stop();
    }

    let launch = build_maestroclaw_launch(&session)?;
    // Redact command arguments for UI display to avoid leaking secrets
    let command_preview = session
        .command
        .clone()
        .filter(|value| !value.trim().is_empty())
        .and_then(|cmd| cmd.split_whitespace().next().map(|first_word| first_word.to_string()))
        .unwrap_or_else(|| fallback_maestroclaw_command(&session));
    let claw_session = ClawSession {
        id: session.session_id.clone(),
        tool: session.tool.clone().unwrap_or_else(|| "shell".to_string()),
        model: None,
        status: ClawSessionStatus::Starting,
        iteration: 1,
        started_at: chrono::Utc::now(),
        tokens_used: 0,
        cost_estimate: 0.0,
        provider_profile: "maestro_runtime".to_string(),
        analysis_provider: "standalone_leindex".to_string(),
        memory_provider: "standalone_nexus".to_string(),
        suppression_policy: serde_json::to_string(
            &managed_cli_overlap_profile(session.tool.as_deref().unwrap_or("shell"))
                .suppression_policy(),
        )
        .unwrap_or_else(|_| "{}".to_string()),
        cli_overlap_profile: serde_json::to_string(&managed_cli_overlap_profile(
            session.tool.as_deref().unwrap_or("shell"),
        ))
        .unwrap_or_else(|_| "{}".to_string()),
    };

    let mut runtime = ClawLoop::launch(claw_session.clone(), launch)?;
    let (cols, rows) = crossterm::terminal::size().unwrap_or((160, 48));
    let _ = runtime.resize(rows, cols);

    app.maestroclaw_pane.selected_session = app
        .sessions
        .iter()
        .position(|candidate| candidate.session_id == session.session_id);
    app.maestroclaw_pane.claw_session = Some(claw_session);
    app.maestroclaw_pane.agent_output.clear();
    app.maestroclaw_pane.output_scroll = 0;
    app.maestroclaw_pane.user_input.clear();
    app.maestroclaw_pane.input_cursor = 0;
    app.maestroclaw_pane.view_mode = ClawViewMode::Agent;
    app.maestroclaw_runtime = Some(runtime);

    push_maestroclaw_lines(
        app,
        vec![
            AgentOutputLine {
                timestamp: chrono::Utc::now(),
                content: format!("Attached MaestroClaw to '{}'", session.title),
                line_type: OutputLineType::SystemMessage,
            },
            AgentOutputLine {
                timestamp: chrono::Utc::now(),
                content: format!("launch {}", command_preview),
                line_type: OutputLineType::ToolCall,
            },
        ],
    );
    app.status_message = format!("MaestroClaw attached to '{}'", session.title);
    Ok(())
}

fn submit_maestroclaw_prompt(app: &mut App) {
    let prompt = app.maestroclaw_pane.user_input.trim().to_string();
    if prompt.is_empty() {
        app.status_message = "Type a prompt before sending it to MaestroClaw".to_string();
        return;
    }

    if app.maestroclaw_runtime.is_none() {
        app.status_message = "No live MaestroClaw session is running".to_string();
        return;
    }

    push_maestroclaw_lines(
        app,
        vec![AgentOutputLine {
            timestamp: chrono::Utc::now(),
            content: prompt.clone(),
            line_type: OutputLineType::UserInput,
        }],
    );

    let submit_result = {
        let runtime = app
            .maestroclaw_runtime
            .as_mut()
            .expect("runtime checked above");
        runtime.submit(&prompt)
    };

    match submit_result {
        Ok(()) => {
            app.maestroclaw_pane.user_input.clear();
            app.maestroclaw_pane.input_cursor = 0;
            app.status_message = "Prompt sent to MaestroClaw session".to_string();
        }
        Err(err) => {
            push_maestroclaw_lines(
                app,
                vec![AgentOutputLine {
                    timestamp: chrono::Utc::now(),
                    content: format!("submit failed: {}", err),
                    line_type: OutputLineType::Error,
                }],
            );
            app.status_message = format!("Failed to send prompt: {}", err);
        }
    }
}

impl App {
    pub fn create_session_manager(
        &mut self,
        svc: &leindex_core::memory::MemoryService,
    ) -> Option<leindex_core::memory::session_manager::SessionManager> {
        let mut manager = match leindex_core::memory::session_manager::SessionManager::new(svc.clone()) {
            Ok(m) => m,
            Err(e) => {
                self.status_message = format!("Failed to create session manager: {}", e);
                return None;
            }
        };

        if let Some(lsp_manager) = self.lsp_manager.clone() {
            manager = manager.with_lsp_manager(lsp_manager);
        } else if let Some(storage) = self.storage_backend.as_ref() {
            let lsp_manager = leindex_core::memory::lsp_manager::LspManager::new((**storage).clone());
            self.lsp_manager = Some(lsp_manager.clone());
            manager = manager.with_lsp_manager(lsp_manager);
        }
        Some(manager)
    }

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
            memory_graph_enabled: false,
            memory_graph_selection: 0,
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
            capabilities_section: Some(crate::tabs::CapabilitiesSection::CronJobs),
            // Phase 3: Capabilities services
            cron_jobs: Vec::new(),
            cron_job_state: ratatui::widgets::ListState::default(),
            mcp_manager: McpManager::new(),
            sandbox_manager: SandboxManager::new(SecurityPolicy::default()),
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
            omp_manager: if is_omp_available() {
                Some(OmpAgentManager::new(None))
            } else {
                None
            },
            hot_cache: crate::maesterclaw::HotCache::new(),
            maestroclaw_pane: crate::maesterclaw::MaestroClawPane::new(
                load_maestroclaw_workspace_dir(),
            ),
            maestroclaw_runtime: None,
            tracklens_pane: TrackLensPane::new(),
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
                    .map(|m| {
                        let access_history = parse_memory_access_history(m.metadata.as_ref());
                        let accessed_by =
                            parse_memory_accessed_by(m.metadata.as_ref(), &access_history);
                        let access_count =
                            parse_memory_access_count(m.metadata.as_ref(), &access_history);
                        let related_memory_ids = parse_memory_related_ids(m.metadata.as_ref());
                        let nexus_runtime_state = parse_memory_runtime_state(m.metadata.as_ref());
                        let nexus_scope = parse_memory_scope(m.metadata.as_ref());

                        MemoryInfo {
                            id: m.id,
                            content: m.content.clone(),
                            category: m.category.to_string(),
                            summary: m.summary.clone(),
                            importance: format!("{:?}", m.importance).to_lowercase(),
                            source: m.source.clone(),
                            session_id: m.session_id.clone(),
                            project_id: m.project_id.map(|id| id.to_string()),
                            track_id: m.track_id.map(|id| id.to_string()),
                            created_at: m.created_at.to_rfc3339(),
                            expires_at: m.expires_at.map(|ts| ts.to_rfc3339()),
                            last_accessed: m.last_accessed.map(|ts| ts.to_rfc3339()),
                            access_count,
                            accessed_by,
                            tags: m.tags.clone().unwrap_or_default(),
                            is_expanded: false,
                            similarity_score: parse_memory_similarity_score(m.metadata.as_ref()),
                            related_memory_ids,
                            nexus_runtime_state,
                            nexus_scope,
                            stored_by: parse_memory_stored_by(
                                m.source.as_deref(),
                                m.metadata.as_ref(),
                            ),
                            access_history,
                        }
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

            // Poll orchestrate sessions (Phase 6: Agentic Loop Integration)
            self.conductor.poll_observed_sessions_sync();
            self.conductor.sync_orchestrate_sessions_blocking();

            // Poll OMP agent status for active track
            if let Some(ref manager) = self.omp_manager {
                if let Some(track_id) = &self.conductor.state.current_track {
                    if let Ok(status) = manager.get_agent_status_sync(track_id) {
                        // Use OmpWorkerStatus methods to check health and display status
                        let worker_status: OmpWorkerStatus = status;
                        if worker_status.is_healthy() {
                            debug!(
                                "OMP worker healthy for track {}: model={}, uptime={}s",
                                track_id, worker_status.model, worker_status.uptime_secs
                            );
                        }
                        // Update status in conductor state for display
                        self.conductor.state.omp_agent_status = Some(worker_status);
                    }
                }
            }
            // Fetch memories for the active track
            if let Some(_track_id) = &self.conductor.state.current_track {
                if let Ok(memories) = svc.list_memories(10) {
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
                            .or_default()
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
            let mut dirs_to_visit: Vec<(std::path::PathBuf, usize)> = vec![(path.to_path_buf(), 0)];

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
                                    if !detected.contains(&"typescript-language-server".to_string())
                                    {
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

/// Cycle to the next theme and save to config
fn cycle_theme(app: &mut App) {
    let current_theme = app.config.theme.to_lowercase();
    let theme_names: Vec<&str> = THEMES.iter().map(|(id, _)| *id).collect();

    // Find current theme index
    let current_idx = theme_names
        .iter()
        .position(|t| t.to_lowercase() == current_theme)
        .unwrap_or(0);

    // Cycle to next theme
    let next_idx = (current_idx + 1) % theme_names.len();
    let next_theme = theme_names[next_idx].to_string();

    // Update config
    app.config.theme = next_theme.clone();

    // Save to config file
    if let Err(e) = app.config.save() {
        app.toast_queue
            .error(format!("Failed to save theme: {}", e));
    } else {
        // Find the display name for the toast
        let display_name = THEMES
            .iter()
            .find(|(id, _)| *id == next_theme)
            .map(|(_, label)| *label)
            .unwrap_or(&next_theme);
        app.toast_queue.success(format!("Theme: {}", display_name));
    }
}

fn handle_maestroclaw_action(app: &mut App, action: crate::maesterclaw::MaestroClawAction) -> bool {
    use crate::maesterclaw::MaestroClawAction;

    match action {
        MaestroClawAction::None => false,
        MaestroClawAction::FocusChanged | MaestroClawAction::Navigate => {
            app.maestroclaw_pane.sync_sessions(app.sessions.len());
            true
        }
        MaestroClawAction::NewSession => {
            app.input_mode = InputMode::NewSessionTitle;
            app.new_session_title = "MaestroClaw Session".to_string();
            app.status_message = "Creating a new MaestroClaw session".to_string();
            true
        }
        MaestroClawAction::OpenSelected => {
            app.maestroclaw_pane.sync_sessions(app.sessions.len());
            if let Some(session) = app
                .maestroclaw_pane
                .selected_session
                .and_then(|idx| app.sessions.get(idx))
                .cloned()
            {
                if let Err(err) = start_maestroclaw_session(app, session.clone()) {
                    app.status_message = format!(
                        "Failed to launch MaestroClaw session '{}': {}",
                        session.title, err
                    );
                }
            } else {
                app.status_message = "No MaestroClaw session selected".to_string();
            }
            true
        }
        MaestroClawAction::StartSetup | MaestroClawAction::RepairBootstrap => {
            app.maestroclaw_pane.activate_wizard();
            app.status_message = "Opening MaestroClaw setup walkthrough".to_string();
            true
        }
        MaestroClawAction::OpenSessionBrowser => {
            // Populate the browser from the app's session list
            let entries: Vec<crate::maesterclaw::SessionEntry> = app
                .sessions
                .iter()
                .map(|s| {
                    let last_active = s
                        .last_accessed_at
                        .map(|dt| {
                            let now = chrono::Utc::now();
                            let diff = now.signed_duration_since(dt);
                            if diff.num_minutes() < 1 {
                                "just now".to_string()
                            } else if diff.num_minutes() < 60 {
                                format!("{}m ago", diff.num_minutes())
                            } else if diff.num_hours() < 24 {
                                format!("{}h ago", diff.num_hours())
                            } else {
                                format!("{}d ago", diff.num_days())
                            }
                        })
                        .unwrap_or_else(|| "unknown".to_string());
                    crate::maesterclaw::SessionEntry {
                        id: s.session_id.clone(),
                        title: s.title.clone(),
                        preview: s.command.as_deref().unwrap_or("(no command)").to_string(),
                        source: s.tool.as_deref().unwrap_or("unknown").to_string(),
                        last_active,
                        turn_count: 0,
                    }
                })
                .collect();
            app.maestroclaw_pane.load_session_entries(entries);
            app.maestroclaw_pane.activate_session_browser();
            true
        }
        MaestroClawAction::SessionBrowserSelect => {
            if let Some(session_id) = app.maestroclaw_pane.selected_browser_session_id() {
                if let Some(idx) = app.sessions.iter().position(|s| s.session_id == session_id) {
                    app.maestroclaw_pane.selected_session = Some(idx);
                    if let Some(session) = app.sessions.get(idx).cloned() {
                        app.status_message =
                            format!("Selected session '{}' in MaestroClaw", session.title);
                    }
                }
            }
            true
        }
        MaestroClawAction::SessionBrowserClose => {
            app.maestroclaw_pane.deactivate_session_browser();
            true
        }
        MaestroClawAction::WizardAdvanced
        | MaestroClawAction::WizardBack
        | MaestroClawAction::WizardSelection => true,
        MaestroClawAction::WizardComplete => {
            app.status_message = match persist_maestroclaw_setup(&mut app.maestroclaw_pane) {
                Ok(message) => message,
                Err(err) => format!("MaestroClaw setup saved with issues: {}", err),
            };
            true
        }
        MaestroClawAction::WizardDismissed => {
            app.status_message = "MaestroClaw setup dismissed".to_string();
            true
        }
    }
}

fn persist_maestroclaw_setup(
    pane: &mut crate::maesterclaw::MaestroClawPane,
) -> anyhow::Result<String> {
    let mut config = maestro_claw::config::Config::load()
        .unwrap_or_else(|_| maestro_claw::config::Config::default());

    if let Some(primary_idx) = pane.wizard.selected_primary_tool {
        if let Some((name, version, binary_path)) = pane.wizard.tool_details.get(primary_idx) {
            config.primary_tool = name.clone();
            if !config.agent_tools.iter().any(|tool| tool.name == *name) {
                config
                    .agent_tools
                    .push(maestro_claw::config::AgentToolConfig {
                        name: name.clone(),
                        binary_path: binary_path.clone(),
                        available: true,
                        version: version.clone(),
                        extra_args: Vec::new(),
                    });
            }
        }
    }

    for (name, version, binary_path) in &pane.wizard.tool_details {
        if let Some(existing) = config
            .agent_tools
            .iter_mut()
            .find(|tool| tool.name == *name)
        {
            existing.binary_path = binary_path.clone();
            existing.version = version.clone();
            existing.available = true;
        } else {
            config
                .agent_tools
                .push(maestro_claw::config::AgentToolConfig {
                    name: name.clone(),
                    binary_path: binary_path.clone(),
                    available: true,
                    version: version.clone(),
                    extra_args: Vec::new(),
                });
        }
    }

    config.cron.enabled = pane.wizard.cron_enabled;
    config.cron.max_run_history = pane.wizard.cron_max_run_history;

    apply_selected_channels_to_config(&mut config, &pane.wizard.selected_channels);

    let workspace_dir = config.workspace_dir.clone();
    maestro_claw::onboard::scaffold_workspace(&workspace_dir, &mut config)?;
    config.ensure_webhook_secret();
    config.bootstrap.setup_timestamp = Some(chrono::Utc::now().timestamp());
    config.bootstrap.setup_version = Some(env!("CARGO_PKG_VERSION").to_string());
    config.setup_status = config.compute_setup_status();
    config.save()?;

    pane.channel_statuses = pane
        .wizard
        .selected_channels
        .iter()
        .map(|channel| crate::maesterclaw::ChannelStatusDisplay {
            channel_type: channel.label().to_string(),
            connected: channel_has_persisted_credentials(&config, *channel),
            last_message: None,
            config_status: if channel_has_persisted_credentials(&config, *channel) {
                "credentials stored".to_string()
            } else {
                "selected without stored credentials".to_string()
            },
        })
        .collect();
    pane.cron_jobs = vec![crate::maesterclaw::CronJobDisplay {
        id: "default-scheduler".to_string(),
        name: "Scheduled automation".to_string(),
        schedule: if config.cron.enabled {
            "enabled".to_string()
        } else {
            "disabled".to_string()
        },
        last_run: None,
        next_run: None,
        status: format!("history {}", config.cron.max_run_history),
    }];

    Ok(format!(
        "MaestroClaw setup complete and saved to {}",
        config.config_path.display()
    ))
}

fn apply_selected_channels_to_config(
    config: &mut maestro_claw::config::Config,
    selected_channels: &std::collections::HashSet<crate::maesterclaw::ChannelType>,
) {
    use crate::maesterclaw::ChannelType;
    use maestro_claw::config::schema::{
        DiscordConfig, MatrixConfig, MattermostConfig, SlackConfig, TelegramConfig, WhatsAppConfig,
    };

    fn parse_env_list(name: &str) -> Vec<String> {
        std::env::var(name)
            .ok()
            .map(|value| {
                value
                    .split(',')
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    // Only clear deselected channels — preserve configs for selected ones
    // (even if env vars are missing, we keep the existing persisted config)
    if !selected_channels.contains(&ChannelType::Telegram) {
        config.channels.telegram = None;
    }
    if !selected_channels.contains(&ChannelType::Discord) {
        config.channels.discord = None;
    }
    if !selected_channels.contains(&ChannelType::Slack) {
        config.channels.slack = None;
    }
    if !selected_channels.contains(&ChannelType::Matrix) {
        config.channels.matrix = None;
    }
    if !selected_channels.contains(&ChannelType::WhatsApp) {
        config.channels.whatsapp = None;
    }
    if !selected_channels.contains(&ChannelType::Mattermost) {
        config.channels.mattermost = None;
    }

    // Now populate selected channels from env vars (overwriting if available)
    if selected_channels.contains(&ChannelType::Telegram) {
        if let Ok(bot_token) = std::env::var("TELEGRAM_BOT_TOKEN") {
            config.channels.telegram = Some(TelegramConfig {
                bot_token,
                allowed_users: parse_env_list("TELEGRAM_ALLOWED_USERS"),
            });
        }
    }

    if selected_channels.contains(&ChannelType::Discord) {
        if let (Ok(bot_token), Ok(guild_id)) = (
            std::env::var("DISCORD_BOT_TOKEN"),
            std::env::var("DISCORD_GUILD_ID"),
        ) {
            config.channels.discord = Some(DiscordConfig {
                bot_token,
                guild_id,
                allowed_users: parse_env_list("DISCORD_ALLOWED_USERS"),
            });
        }
    }

    if selected_channels.contains(&ChannelType::Slack) {
        if let (Ok(bot_token), Ok(app_token)) = (
            std::env::var("SLACK_BOT_TOKEN"),
            std::env::var("SLACK_APP_TOKEN"),
        ) {
            config.channels.slack = Some(SlackConfig {
                bot_token,
                app_token,
                allowed_users: parse_env_list("SLACK_ALLOWED_USERS"),
            });
        }
    }

    if selected_channels.contains(&ChannelType::Matrix) {
        if let (Ok(homeserver_url), Ok(access_token)) = (
            std::env::var("MATRIX_HOMESERVER_URL"),
            std::env::var("MATRIX_ACCESS_TOKEN"),
        ) {
            config.channels.matrix = Some(MatrixConfig {
                homeserver_url,
                access_token,
                bot_user_id: std::env::var("MATRIX_BOT_USER_ID").ok(),
                allowed_users: parse_env_list("MATRIX_ALLOWED_USERS"),
                room_ids: parse_env_list("MATRIX_ROOM_IDS"),
            });
        }
    }

    if selected_channels.contains(&ChannelType::WhatsApp) {
        if let (Ok(bridge_url), Ok(api_token)) = (
            std::env::var("WHATSAPP_BRIDGE_URL"),
            std::env::var("WHATSAPP_API_TOKEN"),
        ) {
            config.channels.whatsapp = Some(WhatsAppConfig {
                bridge_url,
                api_token,
                phone_number_id: std::env::var("WHATSAPP_PHONE_NUMBER_ID").ok(),
                allowed_users: parse_env_list("WHATSAPP_ALLOWED_USERS"),
            });
        }
    }

    if selected_channels.contains(&ChannelType::Mattermost) {
        if let (Ok(server_url), Ok(bot_token)) = (
            std::env::var("MATTERMOST_SERVER_URL"),
            std::env::var("MATTERMOST_TOKEN"),
        ) {
            config.channels.mattermost = Some(MattermostConfig {
                server_url,
                bot_token,
                team_id: std::env::var("MATTERMOST_TEAM_ID").ok(),
                channel_id: std::env::var("MATTERMOST_CHANNEL_ID").ok(),
                allowed_users: parse_env_list("MATTERMOST_ALLOWED_USERS"),
            });
        }
    }
}

fn channel_has_persisted_credentials(
    config: &maestro_claw::config::Config,
    channel: crate::maesterclaw::ChannelType,
) -> bool {
    use crate::maesterclaw::ChannelType;

    match channel {
        ChannelType::Telegram => config.channels.telegram.is_some(),
        ChannelType::Discord => config.channels.discord.is_some(),
        ChannelType::Slack => config.channels.slack.is_some(),
        ChannelType::Matrix => config.channels.matrix.is_some(),
        ChannelType::WhatsApp => config.channels.whatsapp.is_some(),
        ChannelType::Mattermost => config.channels.mattermost.is_some(),
    }
}

fn suspend_fullscreen_app<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    // If we're already inside a Zellij pane, switching the terminal's alternate
    // screen can cause rendering glitches. In that case, let the spawned app
    // manage the terminal as-is.
    if std::env::var("ZELLIJ").is_ok() {
        return Ok(());
    }

    // CRITICAL: Order matters for proper terminal handoff to external TUI apps
    // 1. First show cursor while still in alternate screen
    terminal.show_cursor()?;

    // 2. Leave alternate screen - this returns us to the main screen buffer
    execute!(io::stdout(), LeaveAlternateScreen)?;

    // 3. Clear the main screen to ensure clean state for spawned app
    execute!(io::stdout(), TerminalClear(ClearType::All))?;

    // 4. Move cursor to home position
    execute!(io::stdout(), MoveTo(0, 0))?;

    // 5. NOW disable raw mode - this gives the spawned app proper terminal control
    disable_raw_mode()?;

    // 6. Ensure all output is flushed before returning
    io::stdout().flush()?;

    // 7. Allow terminal to process all sequences (increased delay for reliability)
    std::thread::sleep(std::time::Duration::from_millis(100));

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

fn managed_manifest_temp_path(server_name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "maestro-managed-mcp-{}.toml",
        server_name.replace('/', "-")
    ));
    path
}

fn edit_managed_manifest<B: Backend>(
    terminal: &mut Terminal<B>,
    editor: &str,
    manifest_path: &PathBuf,
    template: &str,
) -> Result<()> {
    fs::write(manifest_path, template)
        .with_context(|| format!("Failed to write manifest {}", manifest_path.display()))?;
    suspend_fullscreen_app(terminal)?;
    let status = std::process::Command::new(editor)
        .arg(manifest_path)
        .status()
        .with_context(|| format!("Failed to launch editor '{}'", editor))?;
    resume_fullscreen_app(terminal)?;
    if !status.success() {
        anyhow::bail!("Editor exited with status {}", status);
    }
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
        sync_maestroclaw_runtime(&mut app);
        terminal.draw(|frame| ui(frame, &mut app))?;

        // Handle MCP refresh task shutdown on quit
        if app.should_quit {
            if let Some(ref task) = app.mcp_refresh_task {
                task.abort();
            }
            if let Some(runtime) = app.maestroclaw_runtime.as_mut() {
                let _ = runtime.stop();
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
        if app.tab_index == tabs::SESSIONS
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
                    if app.lsp_installer.install_output.is_some() {
                        app.lsp_installer.install_output = None;
                        continue;
                    }
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
                                            let Some(manager) = app.create_session_manager(svc) else {
                                                app.is_spawning = false;
                                                continue;
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
                                                    app.maestroclaw_pane.selected_session =
                                                        Some(app.sessions.len().saturating_sub(1));
                                                    if app.tab_index == tabs::MAESTROCLAW {
                                                        if let Err(err) = start_maestroclaw_session(
                                                            &mut app,
                                                            session.clone(),
                                                        ) {
                                                            app.status_message = format!(
                                                                "Session '{}' created but MaestroClaw launch failed: {}",
                                                                session.title, err
                                                            );
                                                        }
                                                    } else {
                                                        app.status_message = format!(
                                                            "Session '{}' created. Press Enter on Sessions tab to attach.",
                                                            session.title
                                                        );
                                                        app.tab_index = tabs::SESSIONS;
                                                    }
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
                                                        if s.group_path.is_none()
                                                            && svc
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
                                                let orig = app.sessions.iter().find(|s| s.session_id == id).cloned();
                                                if let Some(orig) = orig {
                                                    let Some(manager) = app.create_session_manager(svc) else {
                                                        app.input_mode = InputMode::Normal;
                                                        continue;
                                                    };

                                                    match manager.fork_session(
                                                        &id,
                                                        &app.rename_buffer,
                                                        &orig,
                                                    ) {
                                                        Ok(_) => {
                                                            app.status_message = format!(
                                                                "Session forked as {}",
                                                                app.rename_buffer
                                                            );
                                                        }
                                                        Err(e) => {
                                                            app.status_message = format!(
                                                                "Failed to fork session: {}",
                                                                e
                                                            );
                                                        }
                                                    }
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
                                                let Some(manager) = app.create_session_manager(svc) else {
                                                    continue;
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
                                    InputMode::MemoryDetail => {
                                        app.memory_graph_enabled = false;
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::MemoryDetailFocus => {
                                        let targets =
                                            crate::tabs::memory::graph_navigation_targets(&app);
                                        if let Some(target_idx) = targets
                                            .get(
                                                app.memory_graph_selection
                                                    .min(targets.len().saturating_sub(1)),
                                            )
                                            .copied()
                                        {
                                            app.memory_state.select(Some(target_idx));
                                            app.input_mode = InputMode::MemoryDetail;
                                            app.status_message =
                                                "Opened highlighted memory from relationship graph"
                                                    .to_string();
                                        } else {
                                            app.memory_graph_enabled = false;
                                            app.input_mode = InputMode::Normal;
                                        }
                                    }

                                    InputMode::NewMemoryContent => {
                                        // Move to category input
                                        if app.new_memory_content.trim().is_empty() {
                                            app.status_message =
                                                "Memory content cannot be empty".to_string();
                                        } else {
                                            app.input_mode = InputMode::NewMemoryCategory;
                                            app.status_message =
                                                "Enter category (or press Enter for 'general')"
                                                    .to_string();
                                        }
                                    }

                                    InputMode::NewMemoryCategory => {
                                        // Create the memory
                                        let Some(svc) = service.as_ref() else {
                                            app.status_message =
                                                "Error: Memory service not available".to_string();
                                            app.input_mode = InputMode::Normal;
                                            continue;
                                        };

                                        let category = if app.new_memory_category.trim().is_empty()
                                        {
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

                                        match svc.store_memory(
                                            app.new_memory_content.trim(),
                                            mem_category,
                                        ) {
                                            Ok(_) => {
                                                app.status_message = format!(
                                                    "Memory created with category '{}'",
                                                    category
                                                );
                                                app.refresh_from_service(&service);
                                            }
                                            Err(e) => {
                                                app.status_message =
                                                    format!("Failed to create memory: {}", e);
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
                                            McpOption::Install => {
                                                let installer = match ManagedMcpInstaller::new(
                                                    svc.clone(),
                                                    None,
                                                ) {
                                                    Ok(installer) => installer,
                                                    Err(e) => {
                                                        app.status_message =
                                                            format!("Installer init failed: {}", e);
                                                        app.input_mode = InputMode::Normal;
                                                        app.target_mcp_name = None;
                                                        continue;
                                                    }
                                                };
                                                let template = match installer
                                                    .template(server.as_ref(), Some(&name))
                                                {
                                                    Ok(template) => template,
                                                    Err(e) => {
                                                        app.status_message = format!(
                                                            "Template generation failed: {}",
                                                            e
                                                        );
                                                        app.input_mode = InputMode::Normal;
                                                        app.target_mcp_name = None;
                                                        continue;
                                                    }
                                                };
                                                let manifest_path =
                                                    managed_manifest_temp_path(&name);
                                                if let Err(e) = edit_managed_manifest(
                                                    terminal,
                                                    &app.config.editor,
                                                    &manifest_path,
                                                    &template,
                                                ) {
                                                    app.status_message =
                                                        format!("Managed install cancelled: {}", e);
                                                    app.input_mode = InputMode::Normal;
                                                    app.target_mcp_name = None;
                                                    continue;
                                                }

                                                let manifest_toml =
                                                    match fs::read_to_string(&manifest_path) {
                                                        Ok(text) => text,
                                                        Err(e) => {
                                                            app.status_message = format!(
                                                                "Failed to read manifest: {}",
                                                                e
                                                            );
                                                            app.input_mode = InputMode::Normal;
                                                            app.target_mcp_name = None;
                                                            continue;
                                                        }
                                                    };
                                                let manifest: McpManagedInstallManifest =
                                                    match toml::from_str(&manifest_toml) {
                                                        Ok(manifest) => manifest,
                                                        Err(e) => {
                                                            app.status_message =
                                                                format!("Invalid manifest: {}", e);
                                                            app.input_mode = InputMode::Normal;
                                                            app.target_mcp_name = None;
                                                            continue;
                                                        }
                                                    };

                                                match installer
                                                    .install_from_manifest_str(&manifest_toml)
                                                    .await
                                                {
                                                    Ok(installed) => {
                                                        if manifest.auto_start {
                                                            if let Some(pool) = app.mcp_pool.clone()
                                                            {
                                                                if let Err(error) = pool
                                                                    .start_server_record(&installed)
                                                                    .await
                                                                {
                                                                    app.status_message = format!(
                                                                        "Installed '{}', but start failed: {}",
                                                                        installed.name, error
                                                                    );
                                                                } else {
                                                                    app.status_message = format!(
                                                                        "Installed and started managed MCP '{}'",
                                                                        installed.name
                                                                    );
                                                                }
                                                            } else {
                                                                app.status_message = format!(
                                                                    "Installed managed MCP '{}'",
                                                                    installed.name
                                                                );
                                                            }
                                                        } else {
                                                            app.status_message = format!(
                                                                "Installed managed MCP '{}'",
                                                                installed.name
                                                            );
                                                        }
                                                    }
                                                    Err(e) => {
                                                        app.status_message = format!(
                                                            "Managed install failed: {}",
                                                            e
                                                        );
                                                    }
                                                }
                                                let _ = fs::remove_file(&manifest_path);
                                            }
                                            McpOption::Reinstall => {
                                                let Some(server) = server.as_ref() else {
                                                    app.status_message =
                                                        "MCP server not found".to_string();
                                                    app.input_mode = InputMode::Normal;
                                                    app.target_mcp_name = None;
                                                    continue;
                                                };
                                                if !server.managed {
                                                    app.status_message = format!(
                                                        "MCP '{}' is not managed by Maestro",
                                                        name
                                                    );
                                                    app.input_mode = InputMode::Normal;
                                                    app.target_mcp_name = None;
                                                    continue;
                                                }
                                                if let Some(pool) = app.mcp_pool.clone() {
                                                    let _ = pool.stop_server(&name).await;
                                                }
                                                let installer = match ManagedMcpInstaller::new(
                                                    svc.clone(),
                                                    None,
                                                ) {
                                                    Ok(installer) => installer,
                                                    Err(e) => {
                                                        app.status_message =
                                                            format!("Installer init failed: {}", e);
                                                        app.input_mode = InputMode::Normal;
                                                        app.target_mcp_name = None;
                                                        continue;
                                                    }
                                                };
                                                match installer.reinstall(&name).await {
                                                    Ok(installed) => {
                                                        if let Some(pool) = app.mcp_pool.clone() {
                                                            if let Err(error) = pool
                                                                .start_server_record(&installed)
                                                                .await
                                                            {
                                                                app.status_message = format!(
                                                                    "Reinstalled '{}', but start failed: {}",
                                                                    installed.name, error
                                                                );
                                                            } else {
                                                                app.status_message = format!(
                                                                    "Reinstalled managed MCP '{}'",
                                                                    installed.name
                                                                );
                                                            }
                                                        } else {
                                                            app.status_message = format!(
                                                                "Reinstalled managed MCP '{}'",
                                                                installed.name
                                                            );
                                                        }
                                                    }
                                                    Err(e) => {
                                                        app.status_message = format!(
                                                            "Managed reinstall failed: {}",
                                                            e
                                                        );
                                                    }
                                                }
                                            }
                                            McpOption::Remove => {
                                                if let Some(pool) = app.mcp_pool.clone() {
                                                    let _ = pool.stop_server(&name).await;
                                                }
                                                match svc.delete_mcp_server(&name) {
                                                    Ok(_) => {
                                                        app.status_message = format!(
                                                            "Removed MCP '{}' from pool",
                                                            name
                                                        );
                                                    }
                                                    Err(e) => {
                                                        app.status_message =
                                                            format!("Remove failed: {}", e);
                                                    }
                                                }
                                            }
                                            McpOption::Uninstall => {
                                                let Some(server) = server.as_ref() else {
                                                    app.status_message =
                                                        "MCP server not found".to_string();
                                                    app.input_mode = InputMode::Normal;
                                                    app.target_mcp_name = None;
                                                    continue;
                                                };
                                                if !server.managed {
                                                    app.status_message = format!(
                                                        "MCP '{}' is not managed by Maestro",
                                                        name
                                                    );
                                                    app.input_mode = InputMode::Normal;
                                                    app.target_mcp_name = None;
                                                    continue;
                                                }
                                                if let Some(pool) = app.mcp_pool.clone() {
                                                    let _ = pool.stop_server(&name).await;
                                                }
                                                let installer = match ManagedMcpInstaller::new(
                                                    svc.clone(),
                                                    None,
                                                ) {
                                                    Ok(installer) => installer,
                                                    Err(e) => {
                                                        app.status_message =
                                                            format!("Installer init failed: {}", e);
                                                        app.input_mode = InputMode::Normal;
                                                        app.target_mcp_name = None;
                                                        continue;
                                                    }
                                                };
                                                match installer.uninstall(&name).await {
                                                    Ok(_) => {
                                                        app.status_message = format!(
                                                            "Uninstalled managed MCP '{}'",
                                                            name
                                                        );
                                                    }
                                                    Err(e) => {
                                                        app.status_message = format!(
                                                            "Managed uninstall failed: {}",
                                                            e
                                                        );
                                                    }
                                                }
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
                                        if let Err(e) = app.config.save() {
                                            app.status_message =
                                                format!("Failed to save config: {}", e);
                                        } else {
                                            app.status_message =
                                                format!("Editor set to '{}'", app.config.editor);
                                        }
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::SettingsInstallPath => {
                                        app.config.install_path = app.rename_buffer.clone();
                                        if let Err(e) = app.config.save() {
                                            app.status_message =
                                                format!("Failed to save config: {}", e);
                                        } else {
                                            app.status_message = format!(
                                                "Install path set to '{}'",
                                                app.config.install_path
                                            );
                                        }
                                        app.input_mode = InputMode::Normal;
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
                                                if let Err(e) = app.config.save() {
                                                    app.status_message =
                                                        format!("Failed to save config: {}", e);
                                                } else {
                                                    app.status_message = format!(
                                                        "Editor set to '{}'",
                                                        app.config.editor
                                                    );
                                                }
                                            }
                                            SettingsMenuKind::Theme => {
                                                app.config.theme = id.clone();
                                                if let Err(e) = app.config.save() {
                                                    app.status_message =
                                                        format!("Failed to save config: {}", e);
                                                } else {
                                                    app.status_message = format!(
                                                        "Theme set to '{}'",
                                                        app.config.theme
                                                    );
                                                }
                                            }
                                        }

                                        app.settings_menu_kind = None;
                                        app.settings_menu_items.clear();
                                        app.settings_menu_state.select(Some(0));
                                        app.input_mode = InputMode::Normal;
                                    }
                                    InputMode::LspInstaller => {
                                        let available_lsps =
                                            crate::tabs::lsp_registry::get_available_lsps();
                                        if let Some(selected_lsp) =
                                            available_lsps.get(app.lsp_installer.selected_index)
                                        {
                                            let install_cmd =
                                                crate::tabs::lsp_registry::get_install_command(
                                                    selected_lsp,
                                                );

                                            app.status_message = format!(
                                                "Installing {}... This may take a few minutes.",
                                                selected_lsp.display_name
                                            );
                                            let _ = terminal.draw(|frame| ui(frame, &mut app));

                                            app.lsp_installer.is_installing = true;
                                            let shell = std::env::var("SHELL")
                                                .unwrap_or_else(|_| "/bin/bash".to_string());
                                            let exec_result = std::process::Command::new(&shell)
                                                .arg("-c")
                                                .arg(&install_cmd)
                                                .output();
                                            match exec_result {
                                                Ok(output) => {
                                                    let stdout =
                                                        String::from_utf8_lossy(&output.stdout);
                                                    let stderr =
                                                        String::from_utf8_lossy(&output.stderr);
                                                    let mut combined_output = String::new();

                                                    if !stdout.trim().is_empty() {
                                                        combined_output.push_str("[OUT]\n");
                                                        combined_output.push_str(&stdout);
                                                        combined_output.push('\n');
                                                    }

                                                    if !stderr.trim().is_empty() {
                                                        combined_output.push_str("[ERR]\n");
                                                        combined_output.push_str(&stderr);
                                                        combined_output.push('\n');
                                                    }

                                                    if output.status.success() {
                                                        app.status_message = format!(
                                                            "{} installed successfully!",
                                                            selected_lsp.display_name
                                                        );
                                                        combined_output.push_str(&format!(
                                                            "SUCCESS: {} installed successfully.",
                                                            selected_lsp.display_name
                                                        ));
                                                    } else {
                                                        app.status_message = format!(
                                                            "Installation of {} failed with exit code: {:?}",
                                                            selected_lsp.display_name,
                                                            output.status.code()
                                                        );
                                                        combined_output.push_str(&format!(
                                                            "FAILED: {} install exited with code {:?}.",
                                                            selected_lsp.display_name,
                                                            output.status.code()
                                                        ));
                                                    }

                                                    if combined_output.trim().is_empty() {
                                                        combined_output.push_str(
                                                            "No installer output captured.",
                                                        );
                                                    }

                                                    app.lsp_installer.install_output =
                                                        Some(combined_output);
                                                }
                                                Err(e) => {
                                                    app.status_message =
                                                        format!("Failed to start install: {}", e);
                                                    app.lsp_installer.install_output = Some(format!(
                                                        "FAILED: could not start install command.\n{}",
                                                        e
                                                    ));
                                                }
                                            }
                                            app.lsp_installer.is_installing = false;
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
                                                        category,
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
                                                    "Session moved to group".to_string();
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
                                    InputMode::MemoryDetail | InputMode::MemoryDetailFocus => {
                                        // Exit memory detail view - handled above
                                    }
                                    InputMode::NewMemoryContent | InputMode::NewMemoryCategory => {
                                        app.new_memory_content.clear();
                                        app.new_memory_category.clear();
                                        app.status_message =
                                            "Memory creation cancelled".to_string();
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
                                        let tools = [
                                            "claude", "gemini", "shell", "codex", "opencode",
                                            "amp", "qwen", "pi", "omp", "iflow",
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
                                                    app.status_message =
                                                        "Group deleted".to_string();
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
                                InputMode::Normal if app.tab_index == tabs::DASHBOARD => {
                                    app.dash_focus = match app.dash_focus {
                                        DashFocus::Sessions => DashFocus::Mcp,
                                        DashFocus::Mcp => DashFocus::Tabs,
                                        DashFocus::Tabs => DashFocus::Sessions,
                                    };
                                }
                                InputMode::Normal if app.tab_index == tabs::CONDUCTOR => {
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
                                InputMode::Normal if app.tab_index == tabs::DASHBOARD => {
                                    app.dash_focus = match app.dash_focus {
                                        DashFocus::Sessions => DashFocus::Tabs,
                                        DashFocus::Mcp => DashFocus::Sessions,
                                        DashFocus::Tabs => DashFocus::Mcp,
                                    };
                                }
                                InputMode::Normal if app.tab_index == tabs::CONDUCTOR => {
                                    app.conductor.output_focused = !app.conductor.output_focused;
                                }
                                _ => {}
                            },
                            KeyCode::Down if app.tab_index != tabs::CONDUCTOR => {
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
                                    app.mcp_menu_option = app.mcp_menu_option.next();
                                } else if app.input_mode == InputMode::LspInstaller {
                                    let count =
                                        crate::tabs::lsp_registry::get_available_lsps().len();
                                    if app.lsp_installer.selected_index < count.saturating_sub(1) {
                                        app.lsp_installer.selected_index += 1;
                                    } else {
                                        app.lsp_installer.selected_index = 0;
                                    }
                                } else if app.input_mode == InputMode::DiagnosticView {
                                    let count = app.lsp_diagnostics_cache.len();
                                    if count > 0
                                        && app.diagnostic_view.selected_index
                                            < count.saturating_sub(1)
                                    {
                                        app.diagnostic_view.selected_index += 1;
                                    } else {
                                        app.diagnostic_view.selected_index = 0;
                                    }
                                } else if app.preview_focused {
                                    app.preview_scroll = app.preview_scroll.saturating_add(1);
                                } else if app.tab_index == tabs::DASHBOARD {
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
                                } else if app.tab_index == tabs::PROJECTS {
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
                                } else if app.tab_index == tabs::SESSIONS {
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
                                } else if app.tab_index == tabs::MEMORY {
                                    // Memory
                                    if app.memory_graph_enabled
                                        && app.input_mode == InputMode::MemoryDetailFocus
                                    {
                                        let targets =
                                            crate::tabs::memory::graph_navigation_targets(&app);
                                        if !targets.is_empty() {
                                            app.memory_graph_selection = if app
                                                .memory_graph_selection
                                                >= targets.len().saturating_sub(1)
                                            {
                                                0
                                            } else {
                                                app.memory_graph_selection + 1
                                            };
                                        }
                                    } else {
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
                                    }
                                } else if app.tab_index == tabs::MAESTROCLAW {
                                    let action =
                                        app.maestroclaw_pane.handle_key_with_session_count(
                                            KeyCode::Down,
                                            app.sessions.len(),
                                        );
                                    let _ = handle_maestroclaw_action(&mut app, action);
                                } else if app.tab_index == tabs::SETTINGS {
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
                            KeyCode::Up if app.tab_index != tabs::CONDUCTOR => {
                                if app.input_mode == InputMode::McpMenu {
                                    app.mcp_menu_option = app.mcp_menu_option.previous();
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
                                    let count =
                                        crate::tabs::lsp_registry::get_available_lsps().len();
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
                                        app.diagnostic_view.selected_index =
                                            count.saturating_sub(1);
                                    }
                                } else if app.preview_focused {
                                    app.preview_scroll = app.preview_scroll.saturating_sub(1);
                                } else if app.tab_index == tabs::DASHBOARD {
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
                                } else if app.tab_index == tabs::PROJECTS {
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
                                } else if app.tab_index == tabs::SESSIONS {
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
                                } else if app.tab_index == tabs::MEMORY {
                                    // Memory
                                    if app.memory_graph_enabled
                                        && app.input_mode == InputMode::MemoryDetailFocus
                                    {
                                        let targets =
                                            crate::tabs::memory::graph_navigation_targets(&app);
                                        if !targets.is_empty() {
                                            app.memory_graph_selection =
                                                if app.memory_graph_selection == 0 {
                                                    targets.len().saturating_sub(1)
                                                } else {
                                                    app.memory_graph_selection - 1
                                                };
                                        }
                                    } else {
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
                                    }
                                } else if app.tab_index == tabs::MAESTROCLAW {
                                    let action =
                                        app.maestroclaw_pane.handle_key_with_session_count(
                                            KeyCode::Up,
                                            app.sessions.len(),
                                        );
                                    let _ = handle_maestroclaw_action(&mut app, action);
                                } else if app.tab_index == tabs::SETTINGS {
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
                                if app.tab_index == tabs::DASHBOARD {
                                    match app.dash_focus {
                                        DashFocus::Sessions => app.dash_focus = DashFocus::Mcp,
                                        DashFocus::Mcp => app.dash_focus = DashFocus::Tabs,
                                        DashFocus::Tabs => {
                                            app.tab_index = tabs::MAESTROCLAW;
                                            app.dash_focus = DashFocus::Sessions;
                                        }
                                    };
                                } else if app.tab_index == tabs::MEMORY
                                    && app.memory_graph_enabled
                                    && matches!(
                                        app.input_mode,
                                        InputMode::MemoryDetail | InputMode::MemoryDetailFocus
                                    )
                                {
                                    app.input_mode = match app.input_mode {
                                        InputMode::MemoryDetail => InputMode::MemoryDetailFocus,
                                        _ => InputMode::MemoryDetail,
                                    };
                                    app.status_message =
                                        if app.input_mode == InputMode::MemoryDetailFocus {
                                            "Memory relationship graph focused".to_string()
                                        } else {
                                            "Returned focus to memory list".to_string()
                                        };
                                } else if app.tab_index == tabs::MAESTROCLAW
                                    && !app.maestroclaw_pane.is_session_browser_active()
                                {
                                    let action =
                                        app.maestroclaw_pane.handle_key_with_session_count(
                                            KeyCode::Tab,
                                            app.sessions.len(),
                                        );
                                    let _ = handle_maestroclaw_action(&mut app, action);
                                } else {
                                    let tab_count = tabs::all_titles().len();
                                    app.tab_index = (app.tab_index + 1) % tab_count;
                                    app.preview_focused = false;
                                }
                            }
                            (KeyModifiers::SHIFT, KeyCode::BackTab) | (_, KeyCode::BackTab)
                                if app.tab_index != tabs::ANALYSIS =>
                            {
                                if app.tab_index == tabs::DASHBOARD {
                                    match app.dash_focus {
                                        DashFocus::Sessions => {
                                            app.tab_index = tabs::TRACKLENS;
                                            app.dash_focus = DashFocus::Sessions;
                                        }
                                        DashFocus::Mcp => app.dash_focus = DashFocus::Sessions,
                                        DashFocus::Tabs => app.dash_focus = DashFocus::Mcp,
                                    };
                                } else if app.tab_index == tabs::MAESTROCLAW
                                    && !app.maestroclaw_pane.is_session_browser_active()
                                {
                                    let action =
                                        app.maestroclaw_pane.handle_key_with_session_count(
                                            KeyCode::BackTab,
                                            app.sessions.len(),
                                        );
                                    let _ = handle_maestroclaw_action(&mut app, action);
                                } else if app.tab_index == tabs::MEMORY
                                    && app.memory_graph_enabled
                                    && matches!(
                                        app.input_mode,
                                        InputMode::MemoryDetail | InputMode::MemoryDetailFocus
                                    )
                                {
                                    app.input_mode = match app.input_mode {
                                        InputMode::MemoryDetail => InputMode::MemoryDetailFocus,
                                        _ => InputMode::MemoryDetail,
                                    };
                                } else {
                                    app.tab_index = if app.tab_index == tabs::DASHBOARD {
                                        tabs::SETTINGS
                                    } else {
                                        app.tab_index - 1
                                    };
                                    app.preview_focused = false;
                                }
                            }

                            // 2. Conductor Specific Alt Keys (must be before global tab switching)
                            (KeyModifiers::ALT, KeyCode::Char('1'))
                                if app.tab_index == tabs::CONDUCTOR =>
                            {
                                app.conductor.details_mode =
                                    crate::conductor::model::DetailsViewMode::Details;
                            }
                            (KeyModifiers::ALT, KeyCode::Char('2'))
                                if app.tab_index == tabs::CONDUCTOR =>
                            {
                                app.conductor.details_mode =
                                    crate::conductor::model::DetailsViewMode::Output;
                            }
                            (KeyModifiers::ALT, KeyCode::Char('3'))
                                if app.tab_index == tabs::CONDUCTOR =>
                            {
                                app.conductor.details_mode =
                                    crate::conductor::model::DetailsViewMode::Prompt;
                            }
                            (KeyModifiers::ALT, KeyCode::Char('p'))
                                if app.tab_index == tabs::CONDUCTOR =>
                            {
                                app.conductor.output_focused = !app.conductor.output_focused;
                                app.status_message = if app.conductor.output_focused {
                                    "Output focused. Scroll with Arrows/PgUp/PgDn."
                                } else {
                                    "Tracks focused."
                                }
                                .to_string();
                            }

                            // 3. Global Tab Switching - Alt+N
                            (KeyModifiers::ALT, KeyCode::Char('1')) => {
                                app.tab_index = tabs::DASHBOARD
                            }
                            (KeyModifiers::ALT, KeyCode::Char('2')) => {
                                app.tab_index = tabs::MAESTROCLAW
                            }
                            (KeyModifiers::ALT, KeyCode::Char('3')) => {
                                app.tab_index = tabs::SESSIONS
                            }
                            (KeyModifiers::ALT, KeyCode::Char('4')) => {
                                app.tab_index = tabs::PROJECTS
                            }
                            (KeyModifiers::ALT, KeyCode::Char('5')) => {
                                app.tab_index = tabs::CONDUCTOR
                            }
                            (KeyModifiers::ALT, KeyCode::Char('6')) => app.tab_index = tabs::MEMORY,
                            (KeyModifiers::ALT, KeyCode::Char('7')) => {
                                app.tab_index = tabs::ANALYSIS
                            }
                            (KeyModifiers::ALT, KeyCode::Char('8')) => {
                                app.tab_index = tabs::KRUSTOP
                            }
                            (KeyModifiers::ALT, KeyCode::Char('9')) => app.tab_index = tabs::LSPS,
                            (KeyModifiers::ALT, KeyCode::Char('0')) => {
                                app.tab_index = tabs::SETTINGS
                            }

                            // 4. Conductor Catch-All
                            _ if app.tab_index == tabs::CONDUCTOR => {
                                use crate::conductor::keybindings::{
                                    handle_key_event, ConductorAction,
                                };
                                match handle_key_event(&mut app.conductor, key) {
                                    ConductorAction::Handled => continue,
                                    ConductorAction::Toast { message, level } => {
                                        match level {
                                            crate::toast::ToastLevel::Info => {
                                                app.toast_queue.info(message)
                                            }
                                            crate::toast::ToastLevel::Success => {
                                                app.toast_queue.success(message)
                                            }
                                            crate::toast::ToastLevel::Warning => {
                                                app.toast_queue.warning(message)
                                            }
                                            crate::toast::ToastLevel::Error => {
                                                app.toast_queue.error(message)
                                            }
                                        }
                                        continue;
                                    }
                                    ConductorAction::CycleTheme => {
                                        cycle_theme(&mut app);
                                        continue;
                                    }
                                    ConductorAction::StoreMemory { content, category } => {
                                        if let Some(svc) = service.as_ref() {
                                            match svc.store_memory(&content, category) {
                                                Ok(id) => {
                                                    app.toast_queue.success(format!(
                                                        "Memory stored with ID {}",
                                                        id
                                                    ));
                                                    app.refresh_from_service(&service);
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!(
                                                        "Failed to store memory: {}",
                                                        e
                                                    ));
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
                                                    app.toast_queue
                                                        .success(format!("Memory {} deleted", id));
                                                    app.refresh_from_service(&service);
                                                }
                                                Ok(false) => {
                                                    app.toast_queue.warning(format!(
                                                        "Memory {} not found",
                                                        id
                                                    ));
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!(
                                                        "Failed to delete memory: {}",
                                                        e
                                                    ));
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
                            (KeyModifiers::NONE, KeyCode::Char('s'))
                                if app.tab_index == tabs::CONDUCTOR =>
                            {
                                // Start/Run track
                                use crate::conductor::keybindings::{
                                    handle_key_event, ConductorAction,
                                };
                                match handle_key_event(&mut app.conductor, key) {
                                    ConductorAction::Handled => continue,
                                    ConductorAction::Toast { message, level } => {
                                        match level {
                                            crate::toast::ToastLevel::Info => {
                                                app.toast_queue.info(message)
                                            }
                                            crate::toast::ToastLevel::Success => {
                                                app.toast_queue.success(message)
                                            }
                                            crate::toast::ToastLevel::Warning => {
                                                app.toast_queue.warning(message)
                                            }
                                            crate::toast::ToastLevel::Error => {
                                                app.toast_queue.error(message)
                                            }
                                        }
                                        continue;
                                    }
                                    ConductorAction::CycleTheme => {
                                        cycle_theme(&mut app);
                                        continue;
                                    }
                                    ConductorAction::StoreMemory { content, category } => {
                                        if let Some(svc) = service.as_ref() {
                                            match svc.store_memory(&content, category) {
                                                Ok(id) => {
                                                    app.toast_queue.success(format!(
                                                        "Memory stored with ID {}",
                                                        id
                                                    ));
                                                    app.refresh_from_service(&service);
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!(
                                                        "Failed to store memory: {}",
                                                        e
                                                    ));
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
                                                    app.toast_queue
                                                        .success(format!("Memory {} deleted", id));
                                                    app.refresh_from_service(&service);
                                                }
                                                Ok(false) => {
                                                    app.toast_queue.warning(format!(
                                                        "Memory {} not found",
                                                        id
                                                    ));
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!(
                                                        "Failed to delete memory: {}",
                                                        e
                                                    ));
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

                            (KeyModifiers::NONE, KeyCode::Char('p'))
                                if app.tab_index == tabs::CONDUCTOR =>
                            {
                                // Pause track
                                use crate::conductor::keybindings::{
                                    handle_key_event, ConductorAction,
                                };
                                match handle_key_event(&mut app.conductor, key) {
                                    ConductorAction::Handled => continue,
                                    ConductorAction::Toast { message, level } => {
                                        match level {
                                            crate::toast::ToastLevel::Info => {
                                                app.toast_queue.info(message)
                                            }
                                            crate::toast::ToastLevel::Success => {
                                                app.toast_queue.success(message)
                                            }
                                            crate::toast::ToastLevel::Warning => {
                                                app.toast_queue.warning(message)
                                            }
                                            crate::toast::ToastLevel::Error => {
                                                app.toast_queue.error(message)
                                            }
                                        }
                                        continue;
                                    }
                                    ConductorAction::CycleTheme => {
                                        cycle_theme(&mut app);
                                        continue;
                                    }
                                    ConductorAction::StoreMemory { content, category } => {
                                        if let Some(svc) = service.as_ref() {
                                            match svc.store_memory(&content, category) {
                                                Ok(id) => {
                                                    app.toast_queue.success(format!(
                                                        "Memory stored with ID {}",
                                                        id
                                                    ));
                                                    app.refresh_from_service(&service);
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!(
                                                        "Failed to store memory: {}",
                                                        e
                                                    ));
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
                                                    app.toast_queue
                                                        .success(format!("Memory {} deleted", id));
                                                    app.refresh_from_service(&service);
                                                }
                                                Ok(false) => {
                                                    app.toast_queue.warning(format!(
                                                        "Memory {} not found",
                                                        id
                                                    ));
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!(
                                                        "Failed to delete memory: {}",
                                                        e
                                                    ));
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

                            (KeyModifiers::NONE, KeyCode::Char('r'))
                                if app.tab_index == tabs::CONDUCTOR =>
                            {
                                // Resume track
                                use crate::conductor::keybindings::{
                                    handle_key_event, ConductorAction,
                                };
                                match handle_key_event(&mut app.conductor, key) {
                                    ConductorAction::Handled => continue,
                                    ConductorAction::Toast { message, level } => {
                                        match level {
                                            crate::toast::ToastLevel::Info => {
                                                app.toast_queue.info(message)
                                            }
                                            crate::toast::ToastLevel::Success => {
                                                app.toast_queue.success(message)
                                            }
                                            crate::toast::ToastLevel::Warning => {
                                                app.toast_queue.warning(message)
                                            }
                                            crate::toast::ToastLevel::Error => {
                                                app.toast_queue.error(message)
                                            }
                                        }
                                        continue;
                                    }
                                    ConductorAction::CycleTheme => {
                                        cycle_theme(&mut app);
                                        continue;
                                    }
                                    ConductorAction::StoreMemory { content, category } => {
                                        if let Some(svc) = service.as_ref() {
                                            match svc.store_memory(&content, category) {
                                                Ok(id) => {
                                                    app.toast_queue.success(format!(
                                                        "Memory stored with ID {}",
                                                        id
                                                    ));
                                                    app.refresh_from_service(&service);
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!(
                                                        "Failed to store memory: {}",
                                                        e
                                                    ));
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
                                                    app.toast_queue
                                                        .success(format!("Memory {} deleted", id));
                                                    app.refresh_from_service(&service);
                                                }
                                                Ok(false) => {
                                                    app.toast_queue.warning(format!(
                                                        "Memory {} not found",
                                                        id
                                                    ));
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!(
                                                        "Failed to delete memory: {}",
                                                        e
                                                    ));
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

                            (KeyModifiers::NONE, KeyCode::Char('?'))
                                if app.tab_index == tabs::CONDUCTOR =>
                            {
                                // Show status/help
                                use crate::conductor::keybindings::{
                                    handle_key_event, ConductorAction,
                                };
                                match handle_key_event(&mut app.conductor, key) {
                                    ConductorAction::Handled => continue,
                                    ConductorAction::Toast { message, level } => {
                                        match level {
                                            crate::toast::ToastLevel::Info => {
                                                app.toast_queue.info(message)
                                            }
                                            crate::toast::ToastLevel::Success => {
                                                app.toast_queue.success(message)
                                            }
                                            crate::toast::ToastLevel::Warning => {
                                                app.toast_queue.warning(message)
                                            }
                                            crate::toast::ToastLevel::Error => {
                                                app.toast_queue.error(message)
                                            }
                                        }
                                        continue;
                                    }
                                    ConductorAction::CycleTheme => {
                                        cycle_theme(&mut app);
                                        continue;
                                    }
                                    ConductorAction::StoreMemory { content, category } => {
                                        if let Some(svc) = service.as_ref() {
                                            match svc.store_memory(&content, category) {
                                                Ok(id) => {
                                                    app.toast_queue.success(format!(
                                                        "Memory stored with ID {}",
                                                        id
                                                    ));
                                                    app.refresh_from_service(&service);
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!(
                                                        "Failed to store memory: {}",
                                                        e
                                                    ));
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
                                                    app.toast_queue
                                                        .success(format!("Memory {} deleted", id));
                                                    app.refresh_from_service(&service);
                                                }
                                                Ok(false) => {
                                                    app.toast_queue.warning(format!(
                                                        "Memory {} not found",
                                                        id
                                                    ));
                                                }
                                                Err(e) => {
                                                    app.toast_queue.error(format!(
                                                        "Failed to delete memory: {}",
                                                        e
                                                    ));
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
                                if app.tab_index == tabs::MEMORY {
                                    app.input_mode = InputMode::MemorySearch;
                                }
                            }
                            (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                                if app.tab_index == tabs::MEMORY {
                                    app.memory_query.clear();
                                    app.refresh_from_service(&service);
                                }
                            }
                            (KeyModifiers::NONE, KeyCode::Char('g')) => {
                                if app.tab_index == tabs::MEMORY {
                                    app.memory_graph_enabled = !app.memory_graph_enabled;
                                    if app.memory_graph_enabled {
                                        app.input_mode = InputMode::MemoryDetailFocus;
                                        app.memory_graph_selection = 0;
                                        app.status_message =
                                            "Memory relationship graph enabled and focused"
                                                .to_string();
                                    } else {
                                        app.status_message =
                                            "Memory relationship graph hidden".to_string();
                                        if matches!(
                                            app.input_mode,
                                            InputMode::MemoryDetail | InputMode::MemoryDetailFocus
                                        ) {
                                            app.input_mode = InputMode::Normal;
                                        }
                                    }
                                }
                            }
                            // When session browser is active on MaestroClaw tab, short-circuit
                            // before app-global handlers (n, p, t, q, r, /, etc.) so
                            // type-to-filter works end-to-end. Only unmodified keys (no
                            // CTRL/ALT) are captured; Tab, BackTab, and '?' fall through
                            // to global handlers. Modified shortcuts like Ctrl+C also
                            // fall through.
                            (modifiers, key)
                                if app.tab_index == tabs::MAESTROCLAW
                                    && app
                                        .maestroclaw_pane
                                        .should_route_to_browser(modifiers, key) =>
                            {
                                let action = app
                                    .maestroclaw_pane
                                    .handle_key_with_session_count(key, app.sessions.len());
                                let _ = handle_maestroclaw_action(&mut app, action);
                            }
                            (modifiers, key)
                                if app.tab_index == tabs::MAESTROCLAW
                                    && app.maestroclaw_runtime.is_some()
                                    && !app.maestroclaw_pane.is_wizard_active()
                                    && !app.maestroclaw_pane.is_session_browser_active()
                                    && !modifiers
                                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                            {
                                match key {
                                    KeyCode::Enter => submit_maestroclaw_prompt(&mut app),
                                    KeyCode::Backspace => {
                                        app.maestroclaw_pane.user_input.pop();
                                        app.maestroclaw_pane.input_cursor =
                                            app.maestroclaw_pane.user_input.chars().count();
                                    }
                                    KeyCode::Esc => {
                                        app.maestroclaw_pane.user_input.clear();
                                        app.maestroclaw_pane.input_cursor = 0;
                                        app.status_message =
                                            "Cleared MaestroClaw prompt buffer".to_string();
                                    }
                                    KeyCode::Up => {
                                        app.maestroclaw_pane.output_scroll =
                                            app.maestroclaw_pane.output_scroll.saturating_sub(1);
                                    }
                                    KeyCode::Down => {
                                        app.maestroclaw_pane.output_scroll =
                                            app.maestroclaw_pane.output_scroll.saturating_add(1);
                                    }
                                    KeyCode::PageUp => {
                                        app.maestroclaw_pane.output_scroll =
                                            app.maestroclaw_pane.output_scroll.saturating_sub(8);
                                    }
                                    KeyCode::PageDown => {
                                        app.maestroclaw_pane.output_scroll =
                                            app.maestroclaw_pane.output_scroll.saturating_add(8);
                                    }
                                    KeyCode::Char(c) => {
                                        app.maestroclaw_pane.user_input.push(c);
                                        app.maestroclaw_pane.input_cursor =
                                            app.maestroclaw_pane.user_input.chars().count();
                                    }
                                    _ => {}
                                }
                            }
                            (KeyModifiers::NONE, KeyCode::Char('n')) => {
                                // Memory tab: Start new memory creation
                                // Use n to create a memory when Memory tab is focused
                                if app.tab_index == tabs::MEMORY {
                                    app.new_memory_content.clear();
                                    app.new_memory_category.clear();
                                    app.input_mode = InputMode::NewMemoryContent;
                                    app.status_message =
                                        "Creating new memory - enter content".to_string();
                                } else if app.tab_index == tabs::MAESTROCLAW {
                                    let action =
                                        app.maestroclaw_pane.handle_key_with_session_count(
                                            KeyCode::Char('n'),
                                            app.sessions.len(),
                                        );
                                    let _ = handle_maestroclaw_action(&mut app, action);
                                    app.new_session_path = app
                                        .sessions
                                        .get(app.maestroclaw_pane.selected_session.unwrap_or(0))
                                        .map(|session| session.project_path.clone())
                                        .filter(|path: &String| !path.trim().is_empty())
                                        .or_else(|| {
                                            app.projects
                                                .get(app.project_state.selected().unwrap_or(0))
                                                .map(|project| project.path.clone())
                                        })
                                        .unwrap_or_default();
                                } else {
                                    // New session wizard for other tabs
                                    app.input_mode = InputMode::NewSessionTitle;
                                    // Auto-fill path if a project is selected
                                    if app.tab_index == tabs::PROJECTS {
                                        // Projects Tab
                                        if let Some(i) = app.project_state.selected() {
                                            app.new_session_path = app.projects[i].path.clone();
                                            app.new_session_title =
                                                format!("Chat: {}", app.projects[i].name);
                                        }
                                    }
                                }
                            }
                            (KeyModifiers::NONE, KeyCode::Char(c @ ('b' | 'w'))) => {
                                if app.tab_index == tabs::MAESTROCLAW {
                                    let action =
                                        app.maestroclaw_pane.handle_key_with_session_count(
                                            KeyCode::Char(c),
                                            app.sessions.len(),
                                        );
                                    let _ = handle_maestroclaw_action(&mut app, action);
                                }
                            }
                            (KeyModifiers::ALT, KeyCode::Char('p'))
                                if app.tab_index == tabs::PROJECTS =>
                            {
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
                                if app.tab_index != tabs::CONDUCTOR =>
                            {
                                if app.tab_index == tabs::SESSIONS {
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
                            (_, KeyCode::Char('p')) if app.tab_index != tabs::CONDUCTOR => {
                                if app.tab_index == tabs::PROJECTS {
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
                                if app.tab_index == tabs::PROJECTS {
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
                            (_, KeyCode::Esc) if app.tab_index != tabs::CONDUCTOR => {
                                if app.tab_index == tabs::MAESTROCLAW {
                                    let action =
                                        app.maestroclaw_pane.handle_key_with_session_count(
                                            KeyCode::Esc,
                                            app.sessions.len(),
                                        );
                                    let _ = handle_maestroclaw_action(&mut app, action);
                                } else if app.project_view_open {
                                    app.project_view_open = false;
                                }
                            }
                            (_, KeyCode::Char('r')) if app.tab_index != tabs::CONDUCTOR => {
                                if app.tab_index == tabs::SESSIONS {
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
                                } else if app.tab_index == tabs::MEMORY {
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
                                } else if app.tab_index == tabs::KRUSTOP {
                                    // Ktop tab - manual refresh
                                    if app.ktop_state.is_some() {
                                        app.ktop_state.as_mut().expect("ktop_state checked above").mark_refreshed();
                                        app.status_message = "Krustop refreshed".to_string();
                                    }
                                } else if app.tab_index == tabs::LSPS {
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
                            (_, KeyCode::Char('k')) if app.tab_index != tabs::CONDUCTOR => {
                                if app.tab_index == tabs::SESSIONS {
                                    if let Some(i) = app.session_state.selected() {
                                        if let Some(SessionEntry::Session(s)) =
                                            app.session_entries.get(i)
                                        {
                                            app.target_session_id = Some(s.session_id.clone());
                                            app.input_mode = InputMode::KillConfirm;
                                        }
                                    }
                                } else if app.tab_index == tabs::DASHBOARD
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
                            (_, KeyCode::Char('D') | KeyCode::Char('d'))
                                if app.tab_index != tabs::CONDUCTOR
                                    && app.tab_index != tabs::KRUSTOP
                                    && app.tab_index != tabs::LSPS =>
                            {
                                if app.tab_index == tabs::SESSIONS {
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
                                } else if app.tab_index == tabs::DASHBOARD
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
                                if app.tab_index == tabs::SESSIONS {
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
                                if app.tab_index == tabs::LSPS {
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
                            (_, KeyCode::Char('i')) if app.tab_index == tabs::LSPS => {
                                // LSPs tab - open installer modal
                                app.lsp_installer.is_open = true;
                                app.lsp_installer.selected_index = 0;
                                app.lsp_installer.install_output = None;
                                app.input_mode = InputMode::LspInstaller;
                                app.lsp_installer.is_installing = false;
                                app.status_message =
                                    "LSP Installer - Select an LSP to install".to_string();
                            }
                            // Ktop-specific keybindings (use Alt to avoid conflicts)
                            (KeyModifiers::ALT, KeyCode::Char('p'))
                                if app.tab_index == tabs::KRUSTOP =>
                            {
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
                                if app.tab_index == tabs::KRUSTOP {
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
                                if app.tab_index == tabs::KRUSTOP {
                                    // Ktop tab - decrease refresh rate
                                    if let Some(ref mut ktop) = app.ktop_state {
                                        ktop.set_refresh_interval(
                                            ktop.refresh_interval_secs.saturating_sub(1).max(1),
                                        );
                                        app.status_message = format!(
                                            "Refresh interval: {}s",
                                            ktop.refresh_interval_secs
                                        );
                                    }
                                }
                            }
                            (KeyModifiers::ALT, KeyCode::Tab) => {
                                if app.tab_index == tabs::KRUSTOP {
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
                                if app.tab_index == tabs::KRUSTOP {
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
                            (_, KeyCode::Char('d')) if app.tab_index == tabs::LSPS => {
                                if app.tab_index == tabs::LSPS {
                                    // LSPs tab - open diagnostic detail view
                                    app.diagnostic_view.is_open = true;
                                    app.diagnostic_view.selected_index = 0;
                                    app.input_mode = InputMode::DiagnosticView;
                                    app.status_message =
                                        "Diagnostic Details - Press 'S' to send to agent"
                                            .to_string();
                                }
                            }
                            (_, KeyCode::Char('S')) => {
                                if app.tab_index == tabs::LSPS && app.diagnostic_view.is_open {
                                    // Send diagnostics to agent
                                    let project_path = app
                                        .sessions
                                        .first()
                                        .map(|s| s.project_path.clone())
                                        .unwrap_or_else(|| ".".to_string());
                                    let prompt = generate_agent_prompt(
                                        &app.lsp_diagnostics_cache,
                                        &project_path,
                                    );
                                    // Copy to clipboard
                                    let _ = cli_clipboard::set_contents(prompt.clone());
                                    app.status_message = "Diagnostics copied to clipboard! Paste in your agent session.".to_string();
                                }
                            }
                            (_, KeyCode::Char('u') | KeyCode::Char('U')) => {
                                if app.tab_index == tabs::SESSIONS {
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
                                            app.status_message = "Resume failed".to_string();
                                        }
                                    }
                                }
                            }
                            (KeyModifiers::ALT, KeyCode::Char('d'))
                            | (KeyModifiers::ALT, KeyCode::Char('D'))
                            | (KeyModifiers::NONE, KeyCode::Char('d'))
                                if app.tab_index != tabs::CONDUCTOR
                                    && app.tab_index != tabs::KRUSTOP
                                    && app.tab_index != tabs::LSPS =>
                            {
                                if app.tab_index == tabs::SESSIONS {
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
                                } else if app.tab_index == tabs::DASHBOARD
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
                                } else if app.tab_index == tabs::PROJECTS {
                                    // Project list temporary message
                                    app.status_message =
                                        "Project deletion via TUI coming soon in v2.1".to_string();
                                }
                            }
                            (KeyModifiers::ALT, KeyCode::Char('o')) => {
                                app.tab_index = if app.tab_index == tabs::DASHBOARD {
                                    tabs::SETTINGS
                                } else {
                                    app.tab_index - 1
                                };
                                app.preview_focused = false;
                            }
                            (KeyModifiers::ALT, KeyCode::Char('i')) => {
                                let tab_count = tabs::all_titles().len();
                                app.tab_index = (app.tab_index + 1) % tab_count;
                                app.preview_focused = false;
                            }
                            (_, KeyCode::Down) => {
                                if app.input_mode == InputMode::LspInstaller {
                                    let count =
                                        crate::tabs::lsp_registry::get_available_lsps().len();
                                    if app.lsp_installer.selected_index < count.saturating_sub(1) {
                                        app.lsp_installer.selected_index += 1;
                                    } else {
                                        app.lsp_installer.selected_index = 0;
                                    }
                                } else if app.input_mode == InputMode::DiagnosticView {
                                    let count = app.lsp_diagnostics_cache.len();
                                    if count > 0
                                        && app.diagnostic_view.selected_index
                                            < count.saturating_sub(1)
                                    {
                                        app.diagnostic_view.selected_index += 1;
                                    } else {
                                        app.diagnostic_view.selected_index = 0;
                                    }
                                } else if app.input_mode == InputMode::McpMenu {
                                    app.mcp_menu_option = app.mcp_menu_option.next();
                                } else if app.preview_focused {
                                    app.preview_scroll = app.preview_scroll.saturating_add(1);
                                } else if app.tab_index == tabs::DASHBOARD {
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
                                } else if app.tab_index == tabs::PROJECTS {
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
                                } else if app.tab_index == tabs::SESSIONS {
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
                                } else if app.tab_index == tabs::MEMORY {
                                    // Memory
                                    if app.memory_graph_enabled
                                        && app.input_mode == InputMode::MemoryDetailFocus
                                    {
                                        let targets =
                                            crate::tabs::memory::graph_navigation_targets(&app);
                                        if !targets.is_empty() {
                                            app.memory_graph_selection = if app
                                                .memory_graph_selection
                                                >= targets.len().saturating_sub(1)
                                            {
                                                0
                                            } else {
                                                app.memory_graph_selection + 1
                                            };
                                        }
                                    } else {
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
                                    }
                                } else if app.tab_index == tabs::LSPS {
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
                                } else if app.tab_index == tabs::MAESTROCLAW {
                                    let action =
                                        app.maestroclaw_pane.handle_key_with_session_count(
                                            KeyCode::Enter,
                                            app.sessions.len(),
                                        );
                                    let _ = handle_maestroclaw_action(&mut app, action);
                                } else if app.tab_index == tabs::SETTINGS {
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
                                    let count =
                                        crate::tabs::lsp_registry::get_available_lsps().len();
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
                                        app.diagnostic_view.selected_index =
                                            count.saturating_sub(1);
                                    }
                                } else if app.input_mode == InputMode::McpMenu {
                                    app.mcp_menu_option = app.mcp_menu_option.previous();
                                } else if app.preview_focused {
                                    app.preview_scroll = app.preview_scroll.saturating_sub(1);
                                } else if app.tab_index == tabs::DASHBOARD {
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
                                } else if app.tab_index == tabs::PROJECTS {
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
                                } else if app.tab_index == tabs::SESSIONS {
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
                                } else if app.tab_index == tabs::MEMORY {
                                    // Memory
                                    if app.memory_graph_enabled
                                        && app.input_mode == InputMode::MemoryDetailFocus
                                    {
                                        let targets =
                                            crate::tabs::memory::graph_navigation_targets(&app);
                                        if !targets.is_empty() {
                                            app.memory_graph_selection =
                                                if app.memory_graph_selection == 0 {
                                                    targets.len().saturating_sub(1)
                                                } else {
                                                    app.memory_graph_selection - 1
                                                };
                                        }
                                    } else {
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
                                    }
                                } else if app.tab_index == tabs::LSPS {
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
                                } else if app.tab_index == tabs::MAESTROCLAW {
                                    let action =
                                        app.maestroclaw_pane.handle_key_with_session_count(
                                            KeyCode::Up,
                                            app.sessions.len(),
                                        );
                                    let _ = handle_maestroclaw_action(&mut app, action);
                                } else if app.tab_index == tabs::SETTINGS {
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
                                if app.tab_index == tabs::SESSIONS {
                                    app.input_mode = InputMode::NewGroupTitle;
                                    app.rename_buffer.clear();
                                    app.new_group_category.clear();
                                }
                            }
                            (KeyModifiers::ALT, KeyCode::Char('c')) => {
                                if app.tab_index == tabs::SESSIONS {
                                    app.input_mode = InputMode::NewGroupTitle;
                                    app.rename_buffer.clear();
                                    app.new_group_category.clear();
                                }
                            }
                            (_, KeyCode::Char('m')) => {
                                if app.tab_index == tabs::SESSIONS {
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
                                if app.tab_index == tabs::ANALYSIS {
                                    // Analysis tab
                                    app.input_mode = InputMode::AnalysisPrompt;
                                }
                            }
                            (_, KeyCode::Right) => {
                                if app.tab_index == tabs::PROJECTS && app.preview_focused {
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
                                } else if app.tab_index == tabs::SESSIONS {
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
                                if app.tab_index == tabs::MAESTROCLAW {
                                    let action =
                                        app.maestroclaw_pane.handle_key_with_session_count(
                                            KeyCode::Left,
                                            app.sessions.len(),
                                        );
                                    let _ = handle_maestroclaw_action(&mut app, action);
                                } else if app.tab_index == tabs::PROJECTS && app.preview_focused {
                                    if let Some(current) = &app.project_explorer_path {
                                        let path = std::path::PathBuf::from(current);
                                        if let Some(parent) = path.parent() {
                                            app.project_explorer_path =
                                                Some(parent.to_string_lossy().to_string());
                                            app.project_explorer_selected = 0;
                                        }
                                    }
                                } else if app.tab_index == tabs::SESSIONS {
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
                                if app.tab_index == tabs::PROJECTS && app.preview_focused =>
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
                            (_, KeyCode::Char('e')) if app.tab_index == tabs::PROJECTS => {
                                app.preview_focused = !app.preview_focused;
                            }
                            (_, KeyCode::Enter) if app.tab_index != tabs::CONDUCTOR => {
                                if app.tab_index == tabs::DASHBOARD {
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
                                } else if app.tab_index == tabs::SETTINGS {
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
                                        SettingsOption::Save => match app.config.save() {
                                            Ok(()) => {
                                                app.toast_queue.success("Configuration saved to ~/.config/maestro/config.toml");
                                            }
                                            Err(e) => {
                                                app.toast_queue
                                                    .error(format!("Failed to save config: {}", e));
                                            }
                                        },
                                    }
                                } else if app.tab_index == tabs::MEMORY {
                                    if !app.memories.is_empty() {
                                        app.input_mode = InputMode::MemoryDetail;
                                    }
                                } else if app.tab_index == tabs::PROJECTS {
                                    // Projects Tab - Launch Yazi via maestro-tab
                                    if let Some(i) = app.project_state.selected() {
                                        let project = &app.projects[i].clone();

                                        // Check if Yazi launcher is ready
                                        let status = crate::yazi_launcher::get_status_report();
                                        if !status.is_ready() {
                                            app.status_message = status.status_message();
                                            continue;
                                        }

                                        app.status_message =
                                            format!("Launching Yazi for {}...", project.name);
                                        let _ = terminal.draw(|frame| ui(frame, &mut app));

                                        // Properly handle suspend errors
                                        if let Err(e) = suspend_fullscreen_app(terminal) {
                                            app.status_message =
                                                format!("Failed to suspend TUI: {}", e);
                                            continue;
                                        }

                                        // Small delay to ensure terminal state is synced
                                        std::thread::sleep(std::time::Duration::from_millis(50));

                                        let res = crate::yazi_launcher::launch_yazi(
                                            &project.path,
                                            &project.name,
                                        );

                                        // Resume TUI
                                        if let Err(e) = resume_fullscreen_app(terminal) {
                                            app.status_message =
                                                format!("Failed to resume TUI: {}", e);
                                            continue;
                                        }

                                        match res {
                                            Ok(_) => {
                                                let _ = terminal.clear(); // Ensure screen is clear after Yazi exit
                                                let _ = terminal.draw(|frame| ui(frame, &mut app));
                                                app.status_message = format!(
                                                    "Returned from Yazi for {}.",
                                                    project.name
                                                );
                                            }
                                            Err(e) => {
                                                app.status_message = format!("Error: {}", e);
                                            }
                                        }
                                    }
                                } else if app.tab_index == tabs::SESSIONS {
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

                            (_, KeyCode::Char('s')) if app.tab_index != tabs::CONDUCTOR => {
                                if app.tab_index == tabs::DASHBOARD {
                                    // Dashboard (MCP)
                                    if let Some(i) = app.mcp_state.selected() {
                                        if let Some(mcp) = app.mcp_servers.get(i) {
                                            app.target_mcp_name = Some(mcp.name.clone());
                                            app.input_mode = InputMode::McpMenu;
                                            app.mcp_menu_option = McpOption::Start;
                                        }
                                    }
                                } else if app.tab_index == tabs::LSPS {
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
                                if app.tab_index == tabs::DASHBOARD {
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
                                if app.tab_index == tabs::SESSIONS {
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
                                            app.status_message = "Restart failed".to_string();
                                        }
                                    }
                                } else if app.tab_index == tabs::LSPS {
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

fn generate_agent_prompt(
    diagnostics: &[crate::state::LspDiagnosticDetail],
    project_path: &str,
) -> String {
    let mut lines = vec![
        "Investigate these LSP diagnostics and propose the smallest safe fix.".to_string(),
        format!("Project path: {}", project_path),
        String::new(),
    ];

    if diagnostics.is_empty() {
        lines.push("No diagnostic details are currently cached.".to_string());
        return lines.join("\n");
    }

    for (index, diagnostic) in diagnostics.iter().enumerate() {
        let location = match (diagnostic.line, diagnostic.column) {
            (Some(line), Some(column)) => format!("{}:{}", line, column),
            (Some(line), None) => line.to_string(),
            _ => "?".to_string(),
        };
        let lsp_name = diagnostic.lsp_name.as_deref().unwrap_or("unknown-lsp");
        let session = diagnostic
            .session_title
            .as_deref()
            .or(diagnostic.session_id.as_deref())
            .unwrap_or("unknown-session");

        lines.push(format!(
            "{}. [{}] {} :: {} ({})",
            index + 1,
            diagnostic.severity,
            diagnostic.file_path,
            location,
            lsp_name
        ));
        lines.push(format!("   Session: {}", session));
        lines.push(format!("   Message: {}", diagnostic.message));
        if let Some(source) = diagnostic.source.as_deref() {
            lines.push(format!("   Source: {}", source));
        }
        if let Some(code) = diagnostic.code.as_deref() {
            lines.push(format!("   Code: {}", code));
        }
        lines.push(String::new());
    }

    lines.join("\n")
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
    let is_focused = (app.tab_index == tabs::DASHBOARD && app.dash_focus == DashFocus::Tabs)
        || (app.tab_index == tabs::MAESTROCLAW && app.maestroclaw_pane.is_session_browser_active());
    let tabs = Tabs::new(tabs::all_titles())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(if is_focused {
                    BorderType::Double
                } else {
                    BorderType::Rounded
                })
                .border_style(if is_focused {
                    Style::default().fg(theme.accent_alt).bold()
                } else {
                    Style::default().fg(theme.muted)
                })
                .title(" Maestro Cockpit v2.0 "),
        )
        .select(app.tab_index)
        .highlight_style(Style::default().fg(theme.accent).bold());

    frame.render_widget(tabs, chunks[0]);

    match app.tab_index {
        tabs::DASHBOARD => render_dashboard(frame, chunks[1], app),
        tabs::MAESTROCLAW => {
            app.maestroclaw_pane.sync_sessions(app.sessions.len());
            if app.maestroclaw_pane.wizard.available_tools.is_empty() {
                app.maestroclaw_pane.wizard.detect_tools();
            }
            if app.sessions.is_empty() && app.maestroclaw_pane.should_show_wizard(false) {
                app.maestroclaw_pane.activate_wizard();
            }
            app.maestroclaw_pane.render(frame, chunks[1], app)
        }
        tabs::SESSIONS => render_sessions(frame, chunks[1], app),
        tabs::PROJECTS => render_projects(frame, chunks[1], app),
        tabs::CONDUCTOR => {
            let theme = app.theme();
            // Sync active sessions from Sessions tab into conductor tracks
            app.conductor.sync_sessions_as_tracks(&app.sessions);
            crate::conductor::render_conductor(frame, chunks[1], &mut app.conductor, &theme);
        }
        tabs::MEMORY => render_memory(frame, chunks[1], app),
        tabs::ANALYSIS => render_analysis(frame, chunks[1], app),
        tabs::KRUSTOP => crate::tabs::ktop::render_ktop(frame, chunks[1], app),
        tabs::LSPS => render_lsps(frame, chunks[1], app),
        tabs::SETTINGS => render_settings(frame, app),
        tabs::TRACKLENS => render_tracklens(frame, chunks[1], app),
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
            " Alt+1-0 ",
            Style::default().bg(Color::Cyan).fg(Color::Black),
        ),
        Span::raw(" Tabs  "),
        Span::styled(" n ", Style::default().bg(Color::Green).fg(Color::Black)),
        Span::raw(" New  "),
        Span::styled(" s ", Style::default().bg(theme.accent_alt).fg(Color::Black)),
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

    // Render toast notifications (always on top)
    render_toasts(frame, app);
}

/// Render toast notifications as floating overlays
fn render_toasts(frame: &mut Frame, app: &App) {
    // Remove expired toasts (we do this in render since we don't have a tick event)
    // Note: In a real app you'd want to do this on a timer
    let toasts: Vec<_> = app.toast_queue.iter().collect();

    if toasts.is_empty() {
        return;
    }

    let area = frame.area();
    let max_toasts = 5;
    let toast_height = 3u16;
    let toast_width = 50u16;
    let margin = 1u16;

    // Position toasts in top-right corner
    let start_x = area.right().saturating_sub(toast_width + margin);
    let start_y = area.top() + margin;

    for (i, toast) in toasts.iter().take(max_toasts).enumerate() {
        let y = start_y + (i as u16 * (toast_height + margin));

        // Skip if would overflow
        if y + toast_height > area.bottom() {
            break;
        }

        let toast_area = Rect::new(start_x, y, toast_width, toast_height);

        // Determine style based on level
        let (icon, fg_color, bg_color) = match toast.level {
            crate::toast::ToastLevel::Info => ("ℹ", Color::Cyan, Color::Rgb(0, 60, 80)),
            crate::toast::ToastLevel::Success => ("✓", Color::Green, Color::Rgb(0, 60, 40)),
            crate::toast::ToastLevel::Warning => ("⚠", Color::Yellow, Color::Rgb(80, 60, 0)),
            crate::toast::ToastLevel::Error => ("✗", Color::Red, Color::Rgb(80, 20, 20)),
        };

        // Progress bar
        let progress = toast.progress();
        let progress_width = ((toast_width as f32 * (1.0 - progress)) as u16).max(1);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(fg_color))
            .style(Style::default().bg(bg_color));

        let inner = block.inner(toast_area);
        frame.render_widget(block, toast_area);

        // Render icon and message
        let text = Paragraph::new(Line::from(vec![
            Span::styled(format!("{} ", icon), Style::default().fg(fg_color).bold()),
            Span::styled(
                truncate_str(&toast.message, inner.width as usize - 3),
                Style::default().fg(Color::White),
            ),
        ]));
        frame.render_widget(text, inner);

        // Render progress bar at bottom
        let progress_area = Rect::new(
            toast_area.left() + 1,
            toast_area.bottom() - 1,
            progress_width,
            1,
        );
        let progress_block = Block::default().style(Style::default().fg(fg_color).bg(fg_color));
        frame.render_widget(progress_block, progress_area);
    }
}

/// Truncate a string to fit within a maximum width
fn truncate_str(s: &str, max_width: usize) -> String {
    if s.chars().count() <= max_width {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_width.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
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
            Span::styled(
                format!("{:02}", app.stats.project_count),
                Style::default().fg(Color::Green).bold(),
            ),
            Span::styled(
                "  [Active System Roots]",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  🎯 TRACKS:     ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:02}", app.stats.track_count),
                Style::default().fg(Color::Green).bold(),
            ),
            Span::styled(
                "  [Active Workstreams]",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  🧠 MEMORIES:   ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:02}", app.stats.memory_count),
                Style::default().fg(Color::Green).bold(),
            ),
            Span::styled(
                "  [Context Vectors]",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ⚡ LEINDEX:    ", Style::default().fg(Color::Cyan)),
            Span::styled("HD", Style::default().fg(Color::Yellow).bold()),
            Span::styled(
                "  [Multi-Layer structural cache]",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
    ];
    let stats = Paragraph::new(stats_text).block(stats_block);
    frame.render_widget(stats, left_chunks[0]);

    // Welcome message
    let welcome_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Welcome ")
        .title_style(Style::default().fg(theme.accent_alt));

    // Updated welcome section with multi-layer architecture diagram & ANIMATION
    let anim_char = match (app.frame_count / 10) % 4 {
        0 => "⠋",
        1 => "⠙",
        2 => "⠹",
        _ => "⠸",
    };
    let welcome_color = if (app.frame_count / 20) % 2 == 0 {
        theme.accent_alt
    } else {
        theme.accent
    };

    let welcome_text = vec![
        Line::from(vec![
            Span::styled(
                format!(" {} MAESTRO SYSTEM OVERVIEW ", anim_char),
                Style::default().fg(welcome_color).bold(),
            ),
            Span::styled(
                " [v2.0-beta-5]",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(""),
        Line::from("  [WORKSPACE] ─────▶ [SCANNER] ─────▶ [LEINDEXER]"),
        Line::from("       │                │                │"),
        Line::from("       ▼                ▼                ▼"),
        Line::from("  [CONFIGS]        [TRACKS]         [MEMORY DB]"),
        Line::from("       │                │                │"),
        Line::from("       └──────┬─────────┴────────────────┘"),
        Line::from("              ▼"),
        Line::from(vec![Span::styled(
            "      [ AI AGENT LAYER ]",
            Style::default()
                .fg(Color::LightMagenta)
                .bold()
                .add_modifier(Modifier::DIM),
        )]),
        Line::from("      (Claude / Gemini / Codex / AMP)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  🚀 CAPABILITIES & FEATURES:",
            Style::default().bold().fg(Color::Yellow),
        )]),
        Line::from(vec![
            Span::styled("    ● ", Style::default().fg(Color::Green)),
            Span::styled("Indexing: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("N-Layer vector search via LEANN."),
            Span::styled(
                " (Example: 'scan /path/to/repo')",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(vec![
            Span::styled("    ● ", Style::default().fg(Color::Cyan)),
            Span::styled("Sessions: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Persistent tmux environments."),
            Span::styled(
                " (Example: 'n' to spawn)",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(vec![
            Span::styled("    ● ", Style::default().fg(theme.accent_alt)),
            Span::styled("Analysis: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Structural code intelligence."),
            Span::styled(
                " (Example: 'analyze src/')",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(vec![
            Span::styled("    ● ", Style::default().fg(Color::Blue)),
            Span::styled("Memory:   ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Global cross-project knowledge."),
            Span::styled(
                " (Example: Tab 5)",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Maestro is your autonomous coding cockpit. ",
                Style::default().fg(Color::LightBlue).italic(),
            ),
            Span::styled(
                "Stay playful, build fast!",
                Style::default().fg(Color::Yellow).bold(),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("'/'", Style::default().fg(Color::Yellow).bold()),
            Span::styled(
                " for the Ultimate Command Guide",
                Style::default().fg(Color::DarkGray),
            ),
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
        .border_type(
            if app.tab_index == 0 && app.dash_focus == DashFocus::Sessions {
                BorderType::Double
            } else {
                BorderType::Rounded
            },
        )
        .title(" 🕒 Recent Sessions ")
        .title_style(
            if app.tab_index == 0 && app.dash_focus == DashFocus::Sessions {
                Style::default().fg(Color::Blue).bold()
            } else {
                Style::default().fg(Color::Blue)
            },
        );

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
                        leindex_core::memory::models::SessionStatus::Running => {
                            Span::styled(" ● ", Style::default().fg(Color::Green))
                        }
                        leindex_core::memory::models::SessionStatus::Terminated => {
                            Span::styled(" x ", Style::default().fg(Color::Red))
                        }
                        leindex_core::memory::models::SessionStatus::Waiting => {
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
                            line_spans
                                .push(Span::styled("LSP:", Style::default().fg(Color::DarkGray)));
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
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg)
                .bold(),
        )
        .highlight_symbol(">> ");
    frame.render_stateful_widget(sessions, right_chunks[0], &mut app.dash_session_state);

    // Bottom Right - MCP Pool
    let mcp_block = Block::default()
        .borders(Borders::ALL)
        .border_type(if app.tab_index == 0 && app.dash_focus == DashFocus::Mcp {
            BorderType::Double
        } else {
            BorderType::Rounded
        })
        .title(" 🕹️ Interactive MCP Pool ")
        .title_style(if app.tab_index == 0 && app.dash_focus == DashFocus::Mcp {
            Style::default().fg(theme.accent).bold()
        } else {
            Style::default().fg(theme.accent)
        });

    let mcp_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(right_chunks[1]);

    let mcp_info = Paragraph::new(vec![Line::from(vec![
        Span::styled("Tip: ", Style::default().fg(theme.muted)),
        Span::styled("Tool Search", Style::default().fg(theme.warning).bold()),
        Span::styled(
            " is dynamic via `maestro mcp tool-search` (no full tool listing).",
            Style::default().fg(theme.muted),
        ),
    ])])
    .block(Block::default())
    .wrap(Wrap { trim: true });
    frame.render_widget(mcp_info, mcp_chunks[0]);

    let mcp_items: Vec<ListItem> = app
        .mcp_servers
        .iter()
        .map(|s| {
            let status_color = if s.status == leindex_core::memory::models::McpStatus::Running {
                Color::Green
            } else {
                Color::Red
            };
            ListItem::new(vec![Line::from(vec![
                Span::styled(format!("  {} ", s.name), Style::default().bold()),
                Span::styled(
                    format!(" [{}] ", s.status.to_string()),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    format!(" {} active", s.client_count),
                    Style::default().fg(Color::Gray),
                ),
            ])])
        })
        .collect();

    let mcp_list = List::new(mcp_items)
        .block(mcp_block)
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg)
                .bold(),
        )
        .highlight_symbol(">> ");
    frame.render_stateful_widget(mcp_list, mcp_chunks[1], &mut app.mcp_state);
}

fn session_log_tail(session_name: &str, lines: usize) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};

    // Sanitize session_name to prevent path traversal attacks
    let safe_session_name: String = session_name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let path = format!("{}/.maestro/logs/{}.log", home, safe_session_name);

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
        let items: Vec<ListItem> = app
            .projects
            .iter()
            .map(|p| {
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
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(30, 30, 50))
                    .fg(Color::Yellow)
                    .bold(),
            )
            .highlight_symbol(">> ");
        frame.render_stateful_widget(list, chunks[0], &mut app.project_state);

        // File Preview / "Yazi" Column (Right)
        let preview_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(if app.preview_focused {
                " 📂 File Explorer (Focused) "
            } else {
                " 📂 File Explorer "
            })
            .border_style(if app.preview_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            });

        if let Some(i) = app.project_state.selected() {
            let project = &app.projects[i];
            let current_path = app
                .project_explorer_path
                .clone()
                .unwrap_or_else(|| project.path.clone());
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

                app.explorer_items = dir_entries
                    .iter()
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect();

                for (idx, entry) in dir_entries.iter().enumerate().take(30) {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry.path().is_dir();
                    let icon = if is_dir { "📁" } else { "📄" };
                    let color = if is_dir { Color::Blue } else { Color::White };

                    let style = if app.preview_focused && idx == app.project_explorer_selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(color)
                    };

                    file_items.push(ListItem::new(Line::from(vec![
                        Span::styled(format!("    {} ", icon), style),
                        Span::styled(file_name, style),
                    ])));
                }
                if dir_entries.len() > 30 {
                    file_items.push(ListItem::new(Line::from(vec![Span::styled(
                        format!("    ... and {} more items", dir_entries.len() - 30),
                        Style::default().fg(Color::DarkGray).italic(),
                    )])));
                }
            } else {
                file_items.push(ListItem::new(Span::styled(
                    "  Error reading directory. (Path might not exist or need expansion)",
                    Style::default().fg(Color::Red),
                )));
            }

            let list = List::new(file_items).block(preview_block);
            frame.render_widget(list, chunks[1]);
        } else {
            let para = Paragraph::new(vec![
                Line::from(""),
                Line::from("  Select a project to explore its files."),
                Line::from(""),
                Line::from("  Press Enter to open in:"),
                Line::from(vec![Span::styled(
                    format!("  {} ", app.config.editor.to_uppercase()),
                    Style::default().fg(Color::Green).bold(),
                )]),
                Line::from(""),
                Line::from("  (Use 'Space' on installer to change editor)"),
            ])
            .block(preview_block)
            .alignment(Alignment::Center);
            frame.render_widget(para, chunks[1]);
        }
    }
}

fn render_memory(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();
    let is_detail = matches!(
        app.input_mode,
        InputMode::MemoryDetail | InputMode::MemoryDetailFocus
    );
    let is_creating = app.input_mode == InputMode::NewMemoryContent
        || app.input_mode == InputMode::NewMemoryCategory;
    let has_suggestions = !app.hot_cache.is_empty();

    // Determine layout
    let (search_area, hint_area, list_area, input_area, detail_area) = if is_creating {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Min(0),
            ])
            .split(area);
        (chunks[0], None, Some(chunks[2]), Some(chunks[1]), None)
    } else if is_detail || app.memory_graph_enabled {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(chunks[1]);
        (chunks[0], None, Some(main[0]), None, Some(main[1]))
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                if has_suggestions {
                    Constraint::Length(2)
                } else {
                    Constraint::Length(0)
                },
                Constraint::Min(0),
            ])
            .split(area);
        (
            chunks[0],
            if has_suggestions { Some(chunks[1]) } else { None },
            Some(chunks[2]),
            None,
            None,
        )
    };

    // Search bar
    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Memory Search (Ctrl+F, Ctrl+L clear, r refresh, n new, Enter details, g graph) ")
        .title_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.panel_bg));
    let search_text = if app.input_mode == InputMode::MemorySearch {
        format!("{}|", app.memory_query)
    } else {
        app.memory_query.clone()
    };
    frame.render_widget(Paragraph::new(search_text).block(search_block), search_area);

    // Suggestion hints
    if let Some(hint_area) = hint_area {
        render_memory_suggestion_hints(frame, hint_area, app);
    }

    // Memory creation input
    if let Some(input_area) = input_area {
        let input_title = if app.input_mode == InputMode::NewMemoryContent {
            " New Memory Content (Enter to continue, Esc to cancel) "
        } else {
            " Category (general, knowledge, preference, spec, fact, pattern, decision, context, temp, observation) "
        };
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(input_title)
            .title_style(Style::default().fg(theme.accent_alt))
            .border_style(Style::default().fg(theme.accent));
        let input_text = if app.input_mode == InputMode::NewMemoryContent {
            format!("{}|", app.new_memory_content)
        } else {
            format!("{}|", app.new_memory_category)
        };
        frame.render_widget(
            Paragraph::new(input_text)
                .block(input_block)
                .style(Style::default().fg(Color::White)),
            input_area,
        );
    }

    // Memory list
    if let Some(list_area) = list_area {
        render_memory_list(frame, list_area, app);
    }

    // Detail panel
    if let Some(detail_area) = detail_area {
        render_memory_detail(frame, detail_area, app);
    }
}

/// Maximum content preview length in collapsed view
const MEMORY_PREVIEW_LEN: usize = 60;

/// Render the memory list with expandable entries
fn render_memory_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();
    let is_detail = matches!(
        app.input_mode,
        InputMode::MemoryDetail | InputMode::MemoryDetailFocus
    );

    let subtitle = if app.input_mode == InputMode::MemoryDetailFocus {
        "graph focused, Tab returns to list"
    } else if app.memory_graph_enabled {
        "Tab focuses graph, g hides graph"
    } else if is_detail {
        "Enter to view details"
    } else {
        "Enter to view details, Space expand"
    };

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(" Memory Results ({}) ", subtitle))
        .title_style(Style::default().fg(theme.accent_alt))
        .style(Style::default().bg(theme.panel_bg));

    if app.memories.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from("  No memories found."),
            Line::from(""),
            Line::from("  Tip: press 'r' to import system-wide memories."),
            Line::from("  Tip: press 'n' to create a new memory."),
        ];
        frame.render_widget(Paragraph::new(text).block(list_block), area);
        return;
    }

    let items: Vec<ListItem> = app
        .memories
        .iter()
        .map(|m| {
            let expand_icon = if m.is_expanded { " v " } else { " > " };
            let preview = if m.content.chars().count() > MEMORY_PREVIEW_LEN {
                format!(
                    "{}...",
                    m.content.chars().take(MEMORY_PREVIEW_LEN).collect::<String>()
                )
            } else {
                m.content.clone()
            };
            let (cat_color, cat_icon) = memory_category_style(&m.category);
            let importance_indicator = match m.importance.as_str() {
                "critical" => " [!]",
                "high" => " [*]",
                _ => "",
            };

            if m.is_expanded {
                let mut lines = vec![Line::from(vec![
                    Span::styled(expand_icon, Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("[{}{}] ", cat_icon, m.category),
                        Style::default().fg(cat_color).bold(),
                    ),
                    Span::styled(preview, Style::default().fg(Color::White)),
                    Span::styled(importance_indicator, Style::default().fg(Color::Red)),
                ])];

                if let Some(ref summary) = m.summary {
                    lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(
                            format!("Summary: {}", summary),
                            Style::default().fg(Color::DarkGray).italic(),
                        ),
                    ]));
                }

                lines.push(Line::from(vec![
                    Span::raw("      "),
                    Span::styled(
                        format!("Created: {} | Access: {} times", m.created_at, m.access_count),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));

                if !m.tags.is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(
                            format!("Tags: {}", m.tags.join(", ")),
                            Style::default().fg(Color::Cyan),
                        ),
                    ]));
                }

                if !m.stored_by.trim().is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(
                            format!("Agent: {}", m.stored_by),
                            Style::default().fg(Color::Magenta),
                        ),
                    ]));
                }

                if let Some(state) = m.nexus_runtime_state.as_deref() {
                    lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(
                            format!("Runtime: {}", state),
                            Style::default().fg(Color::Green),
                        ),
                    ]));
                }

                ListItem::new(lines)
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(expand_icon, Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("[{}{}] ", cat_icon, m.category),
                        Style::default().fg(cat_color),
                    ),
                    Span::styled(preview, Style::default().fg(Color::White)),
                    Span::styled(importance_indicator, Style::default().fg(Color::Red)),
                ]))
            }
        })
        .collect();

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg)
                .bold(),
        )
        .highlight_symbol(">> ");
    frame.render_stateful_widget(list, area, &mut app.memory_state);
}

/// Render the detail panel with full metadata and vector visualization
fn render_memory_detail(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let selected_idx = app.memory_state.selected().unwrap_or(0);
    let memory = app.memories.get(selected_idx);

    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Memory Details ")
        .title_style(Style::default().fg(theme.accent_alt))
        .style(Style::default().bg(theme.panel_bg));

    let inner = detail_block.inner(area);

    if let Some(m) = memory {
        let detail_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(10)])
            .split(inner);

        render_memory_detail_content(frame, detail_chunks[0], m, &theme);

        if app.memory_graph_enabled {
            render_memory_graph(frame, detail_chunks[1], app, &theme);
        } else {
            render_memory_vector_viz(frame, detail_chunks[1], m, &theme);
        }
    } else {
        frame.render_widget(
            Paragraph::new("No memory selected")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            inner,
        );
    }

    frame.render_widget(detail_block, area);
}

/// Render content and metadata section of the detail panel
fn render_memory_detail_content(
    frame: &mut Frame,
    area: Rect,
    memory: &crate::state::MemoryInfo,
    theme: &crate::theme::Theme,
) {
    let (cat_color, cat_icon) = memory_category_style(&memory.category);

    let mut lines = vec![
        // Header with category and importance
        Line::from(vec![
            Span::styled(
                format!("[{}{}] ", cat_icon, memory.category),
                Style::default().fg(cat_color).bold(),
            ),
            Span::styled(
                format!("[{}]", memory.importance),
                Style::default().fg(memory_importance_color(&memory.importance)),
            ),
            if let Some(score) = memory.similarity_score {
                Span::styled(
                    format!(" [sim: {:.2}]", score),
                    Style::default().fg(Color::Magenta),
                )
            } else {
                Span::raw("")
            },
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Content:",
            Style::default().fg(theme.accent).bold(),
        )),
    ];

    // Wrapped content
    let content_lines = memory_wrap_text(&memory.content, area.width.saturating_sub(2) as usize);
    for line in content_lines {
        lines.push(Line::from(Span::styled(
            line,
            Style::default().fg(Color::White),
        )));
    }

    lines.push(Line::from(""));

    // Summary
    if let Some(ref summary) = memory.summary {
        lines.push(Line::from(Span::styled(
            "Summary:",
            Style::default().fg(theme.accent).bold(),
        )));
        lines.push(Line::from(Span::styled(
            summary.clone(),
            Style::default().fg(Color::DarkGray).italic(),
        )));
        lines.push(Line::from(""));
    }

    // Metadata
    lines.push(Line::from(Span::styled(
        "Metadata:",
        Style::default().fg(theme.accent).bold(),
    )));

    lines.push(Line::from(vec![
        Span::styled("  Created: ", Style::default().fg(Color::DarkGray)),
        Span::styled(&memory.created_at, Style::default().fg(Color::White)),
    ]));

    if let Some(ref expires) = memory.expires_at {
        lines.push(Line::from(vec![
            Span::styled("  Expires: ", Style::default().fg(Color::DarkGray)),
            Span::styled(expires, Style::default().fg(Color::Yellow)),
        ]));
    }

    if let Some(ref accessed) = memory.last_accessed {
        lines.push(Line::from(vec![
            Span::styled("  Last Accessed: ", Style::default().fg(Color::DarkGray)),
            Span::styled(accessed, Style::default().fg(Color::White)),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("  Access Count: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", memory.access_count),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    if let Some(ref source) = memory.source {
        lines.push(Line::from(vec![
            Span::styled("  Source: ", Style::default().fg(Color::DarkGray)),
            Span::styled(source, Style::default().fg(Color::White)),
        ]));
    }

    if let Some(ref session_id) = memory.session_id {
        lines.push(Line::from(vec![
            Span::styled("  Session: ", Style::default().fg(Color::DarkGray)),
            Span::styled(session_id, Style::default().fg(Color::Cyan)),
        ]));
    }

    if let Some(ref project_id) = memory.project_id {
        lines.push(Line::from(vec![
            Span::styled("  Project ID: ", Style::default().fg(Color::DarkGray)),
            Span::styled(project_id, Style::default().fg(Color::White)),
        ]));
    }

    if let Some(ref track_id) = memory.track_id {
        lines.push(Line::from(vec![
            Span::styled("  Track ID: ", Style::default().fg(Color::DarkGray)),
            Span::styled(track_id, Style::default().fg(Color::White)),
        ]));
    }

    // Tags
    if !memory.tags.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Tags: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                memory.tags.join(", "),
                Style::default().fg(Color::Green),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Stored By: ", Style::default().fg(Color::Cyan).bold()),
        Span::styled(
            if memory.stored_by.trim().is_empty() {
                "unknown"
            } else {
                &memory.stored_by
            },
            Style::default().fg(Color::White),
        ),
    ]));

    // Nexus runtime / subconscious
    if memory.nexus_scope.is_some()
        || memory.nexus_runtime_state.is_some()
        || !memory.related_memory_ids.is_empty()
    {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Nexus Runtime / Subconscious:",
            Style::default().fg(theme.accent).bold(),
        )));

        if let Some(scope) = memory.nexus_scope.as_deref() {
            lines.push(Line::from(vec![
                Span::styled("  Scope: ", Style::default().fg(Color::DarkGray)),
                Span::styled(scope, Style::default().fg(Color::Green)),
            ]));
        }

        if let Some(runtime_state) = memory.nexus_runtime_state.as_deref() {
            lines.push(Line::from(vec![
                Span::styled("  Runtime: ", Style::default().fg(Color::DarkGray)),
                Span::styled(runtime_state, Style::default().fg(Color::Magenta)),
            ]));
        }

        if !memory.related_memory_ids.is_empty() {
            let preview = memory
                .related_memory_ids
                .iter()
                .take(12)
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(Line::from(vec![
                Span::styled("  Related IDs: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    if memory.related_memory_ids.len() > 12 {
                        format!("{} ...", preview)
                    } else {
                        preview
                    },
                    Style::default().fg(Color::Cyan),
                ),
            ]));
        }
    }

    // Agent access history
    if !memory.accessed_by.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Accessed By Agents:",
            Style::default().fg(theme.accent).bold(),
        )));
        for agent in &memory.accessed_by {
            lines.push(Line::from(vec![
                Span::styled("  * ", Style::default().fg(Color::DarkGray)),
                Span::styled(agent, Style::default().fg(Color::Magenta)),
            ]));
        }
    }

    // Access history
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Access History:",
        Style::default().fg(theme.accent).bold().underlined(),
    )));

    if memory.access_history.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No detailed access history recorded.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for event in &memory.access_history {
            let mut spans = vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    if event.timestamp.trim().is_empty() {
                        "unknown-time".to_string()
                    } else {
                        event.timestamp.clone()
                    },
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(" ", Style::default()),
                Span::styled(&event.agent_id, Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!(" ({})", event.access_type),
                    Style::default().fg(Color::Gray),
                ),
            ];
            if let Some(tool_used) = &event.tool_used {
                spans.push(Span::styled(
                    format!(" via {}", tool_used),
                    Style::default().fg(Color::Cyan),
                ));
            }
            lines.push(Line::from(spans));
        }
    }

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        area,
    );
}

/// Relationship classification between memories
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MemoryRelationKind {
    NexusLink,
    SessionLink,
    TrackLink,
    ProjectLink,
    AgentLink,
    TagLink,
    CategoryLink,
    Other,
}

fn memory_relation_kind_label(kind: MemoryRelationKind) -> &'static str {
    match kind {
        MemoryRelationKind::NexusLink => "nexus link",
        MemoryRelationKind::SessionLink => "session link",
        MemoryRelationKind::TrackLink => "track link",
        MemoryRelationKind::ProjectLink => "project link",
        MemoryRelationKind::AgentLink => "agent link",
        MemoryRelationKind::TagLink => "tag link",
        MemoryRelationKind::CategoryLink => "category link",
        MemoryRelationKind::Other => "other",
    }
}

fn memory_relation_style(kind: MemoryRelationKind) -> (Color, &'static str) {
    match kind {
        MemoryRelationKind::NexusLink => (Color::Magenta, "N"),
        MemoryRelationKind::SessionLink => (Color::Cyan, "S"),
        MemoryRelationKind::TrackLink => (Color::Blue, "T"),
        MemoryRelationKind::ProjectLink => (Color::Green, "P"),
        MemoryRelationKind::AgentLink => (Color::Yellow, "A"),
        MemoryRelationKind::TagLink => (Color::LightBlue, "#"),
        MemoryRelationKind::CategoryLink => (Color::LightYellow, "C"),
        MemoryRelationKind::Other => (Color::DarkGray, "."),
    }
}

fn memory_relation_rank(kind: MemoryRelationKind) -> u8 {
    match kind {
        MemoryRelationKind::NexusLink => 100,
        MemoryRelationKind::SessionLink => 90,
        MemoryRelationKind::TrackLink => 80,
        MemoryRelationKind::ProjectLink => 70,
        MemoryRelationKind::AgentLink => 60,
        MemoryRelationKind::TagLink => 40,
        MemoryRelationKind::CategoryLink => 30,
        MemoryRelationKind::Other => 10,
    }
}

fn classify_memory_relation(
    selected: &crate::state::MemoryInfo,
    memory: &crate::state::MemoryInfo,
) -> MemoryRelationKind {
    if selected.id == memory.id {
        return MemoryRelationKind::NexusLink;
    }
    if selected.related_memory_ids.contains(&memory.id)
        || memory.related_memory_ids.contains(&selected.id)
    {
        return MemoryRelationKind::NexusLink;
    }
    if selected.session_id.is_some() && selected.session_id == memory.session_id {
        return MemoryRelationKind::SessionLink;
    }
    if selected.track_id.is_some() && selected.track_id == memory.track_id {
        return MemoryRelationKind::TrackLink;
    }
    if selected.project_id.is_some() && selected.project_id == memory.project_id {
        return MemoryRelationKind::ProjectLink;
    }
    if !selected.stored_by.trim().is_empty()
        && selected.stored_by.eq_ignore_ascii_case(&memory.stored_by)
        && !matches!(
            selected.stored_by.to_ascii_lowercase().as_str(),
            "unknown" | "nexus" | "maestro"
        )
    {
        return MemoryRelationKind::AgentLink;
    }
    if selected
        .tags
        .iter()
        .any(|tag| memory.tags.iter().any(|c| c == tag))
    {
        return MemoryRelationKind::TagLink;
    }
    if selected.category == memory.category {
        return MemoryRelationKind::CategoryLink;
    }
    MemoryRelationKind::Other
}

fn memory_graph_navigation_targets(app: &App) -> Vec<usize> {
    if app.memories.is_empty() {
        return Vec::new();
    }
    let selected_idx = app
        .memory_state
        .selected()
        .unwrap_or(0)
        .min(app.memories.len().saturating_sub(1));
    let selected = &app.memories[selected_idx];
    let mut ranked = Vec::new();
    for (idx, memory) in app.memories.iter().enumerate() {
        if idx == selected_idx {
            continue;
        }
        let kind = classify_memory_relation(selected, memory);
        ranked.push((idx, memory_relation_rank(kind), kind));
    }
    ranked.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| app.memories[a.0].category.cmp(&app.memories[b.0].category))
            .then_with(|| app.memories[a.0].content.cmp(&app.memories[b.0].content))
    });
    ranked.into_iter().map(|(idx, _, _)| idx).collect()
}

/// Render the memory relationship graph
fn render_memory_graph(frame: &mut Frame, area: Rect, app: &App, theme: &crate::theme::Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Memory Relationships ")
        .title_style(Style::default().fg(theme.accent));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.memories.is_empty() || inner.height == 0 || inner.width == 0 {
        frame.render_widget(
            Paragraph::new("No memories to visualize.").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let targets = memory_graph_navigation_targets(app);
    if targets.is_empty() {
        frame.render_widget(
            Paragraph::new("No graph targets available.")
                .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let selected_idx = app
        .memory_state
        .selected()
        .unwrap_or(0)
        .min(app.memories.len().saturating_sub(1));
    let graph_cursor = app
        .memory_graph_selection
        .min(targets.len().saturating_sub(1));
    let selected = &app.memories[selected_idx];

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("anchor ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if selected.content.chars().count() > 26 {
                    format!(
                        "{}...",
                        selected.content.chars().take(26).collect::<String>()
                    )
                } else {
                    selected.content.clone()
                },
                Style::default().fg(Color::Cyan).bold(),
            ),
        ]),
        Line::from(vec![
            Span::styled("focus ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if app.input_mode == InputMode::MemoryDetailFocus {
                    "graph navigation"
                } else {
                    "list navigation"
                },
                Style::default().fg(Color::White),
            ),
            Span::styled("  Tab ", Style::default().fg(Color::DarkGray)),
            Span::styled("switch", Style::default().fg(Color::Green)),
            Span::styled("  Enter ", Style::default().fg(Color::DarkGray)),
            Span::styled("open", Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
    ];

    if let Some(scope) = selected.nexus_scope.as_deref() {
        lines.push(Line::from(vec![
            Span::styled("scope ", Style::default().fg(Color::DarkGray)),
            Span::styled(scope, Style::default().fg(Color::Green)),
        ]));
    }

    if let Some(runtime_state) = selected.nexus_runtime_state.as_deref() {
        lines.push(Line::from(vec![
            Span::styled("runtime ", Style::default().fg(Color::DarkGray)),
            Span::styled(runtime_state, Style::default().fg(Color::Magenta)),
        ]));
    }

    if !selected.related_memory_ids.is_empty() {
        let preview = selected
            .related_memory_ids
            .iter()
            .take(8)
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(Line::from(vec![
            Span::styled("nexus ids ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if selected.related_memory_ids.len() > 8 {
                    format!("{} ...", preview)
                } else {
                    preview
                },
                Style::default().fg(Color::Cyan),
            ),
        ]));
    }

    lines.push(Line::from(""));

    // Group related memories by relationship kind
    let mut related_groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for idx in targets.into_iter().skip(1) {
        let memory = &app.memories[idx];
        let relation = memory_relation_kind_label(classify_memory_relation(selected, memory)).to_string();
        related_groups.entry(relation).or_default().push(idx);
    }

    // Cache navigation targets for cursor comparison
    let graph_nav_targets = memory_graph_navigation_targets(app);

    for (relation, indices) in related_groups {
        let kind = match relation.as_str() {
            "nexus link" => MemoryRelationKind::NexusLink,
            "session link" => MemoryRelationKind::SessionLink,
            "track link" => MemoryRelationKind::TrackLink,
            "project link" => MemoryRelationKind::ProjectLink,
            "agent link" => MemoryRelationKind::AgentLink,
            "tag link" => MemoryRelationKind::TagLink,
            "category link" => MemoryRelationKind::CategoryLink,
            _ => MemoryRelationKind::Other,
        };
        let (cat_color, cat_icon) = memory_relation_style(kind);
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} {}", cat_icon, relation),
                Style::default().fg(cat_color).bold(),
            ),
            Span::styled(
                format!(" ({})", indices.len()),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        for idx in indices {
            let memory = &app.memories[idx];
            let cursor_idx = graph_nav_targets
                .iter()
                .position(|candidate| *candidate == idx)
                .unwrap_or(0);
            let is_graph_selected = cursor_idx == graph_cursor;
            let preview = if memory.content.chars().count() > 28 {
                format!(
                    "{}...",
                    memory.content.chars().take(28).collect::<String>()
                )
            } else {
                memory.content.clone()
            };
            let style = if is_graph_selected {
                Style::default().fg(Color::Black).bg(Color::Yellow).bold()
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(vec![
                Span::styled("  |- ", Style::default().fg(cat_color)),
                Span::styled(
                    format!("[{}] ", memory.id),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(preview, style),
            ]));
        }
    }

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        inner,
    );
}

/// Render a simple vector space visualization using ASCII art
fn render_memory_vector_viz(
    frame: &mut Frame,
    area: Rect,
    memory: &crate::state::MemoryInfo,
    theme: &crate::theme::Theme,
) {
    if area.height < 5 || area.width < 10 {
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Vector Space Visualization ")
        .title_style(Style::default().fg(theme.accent));

    let inner = block.inner(area);
    let viz_lines = generate_memory_vector_viz(memory, inner.width, inner.height);
    frame.render_widget(
        Paragraph::new(viz_lines).style(Style::default().fg(Color::White)),
        inner,
    );
    frame.render_widget(block, area);
}

/// Generate ASCII art for vector space visualization
fn generate_memory_vector_viz(
    memory: &crate::state::MemoryInfo,
    width: u16,
    height: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let grid_width = width.saturating_sub(2) as usize;
    let grid_height = height.saturating_sub(2) as usize;

    if grid_width < 10 || grid_height < 3 {
        return vec![Line::from("Too small")];
    }

    let center_x = grid_width / 2;
    let center_y = grid_height / 2;

    for y in 0..grid_height {
        let mut row = String::new();
        for x in 0..grid_width {
            let dx = (x as i32 - center_x as i32).abs();
            let dy = (y as i32 - center_y as i32).abs();
            let dist = (dx * dx + dy * dy) as f32;
            let intensity = memory.similarity_score.unwrap_or(0.8);

            if x == center_x && y == center_y {
                row.push('*');
            } else if dist < (grid_width as f32 * intensity * 0.3).powi(2) {
                row.push('.');
            } else if dist < (grid_width as f32 * 0.5).powi(2) {
                if x % 4 == 0 && y % 2 == 0 {
                    row.push('o');
                } else {
                    row.push(' ');
                }
            } else {
                row.push(' ');
            }
        }
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(Color::DarkGray),
        )));
    }

    if lines.len() > 2 {
        lines.push(Line::from(vec![
            Span::styled("* ", Style::default().fg(Color::Green)),
            Span::styled("Current ", Style::default().fg(Color::DarkGray)),
            Span::styled(". ", Style::default().fg(Color::Cyan)),
            Span::styled("Related ", Style::default().fg(Color::DarkGray)),
            Span::styled("o ", Style::default().fg(Color::DarkGray)),
            Span::styled("Other", Style::default().fg(Color::DarkGray)),
        ]));
    }

    lines
}

/// Render non-intrusive suggestion hints from the hot cache
fn render_memory_suggestion_hints(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();
    let suggestions = app.hot_cache.active_suggestions();
    if suggestions.is_empty() {
        return;
    }
    let hint_text = format!(
        " Suggestions: {} ",
        suggestions
            .iter()
            .take(3)
            .map(|s| s.preview.as_str())
            .collect::<Vec<_>>()
            .join(" | ")
    );
    frame.render_widget(
        Paragraph::new(hint_text).style(
            Style::default()
                .fg(theme.accent_alt)
                .bg(theme.panel_bg),
        ),
        area,
    );
}

/// Get color and icon for a memory category
fn memory_category_style(category: &str) -> (Color, &'static str) {
    match category.to_lowercase().as_str() {
        "general" => (Color::Yellow, ""),
        "knowledge" => (Color::Blue, ""),
        "preference" | "preferences" => (Color::Magenta, ""),
        "specification" | "specifications" => (Color::Cyan, ""),
        "fact" => (Color::Green, ""),
        "pattern" => (Color::LightBlue, ""),
        "decision" => (Color::LightYellow, ""),
        "context" => (Color::Gray, ""),
        "temporary" => (Color::DarkGray, ""),
        "observation" => (Color::LightCyan, ""),
        _ => (Color::White, ""),
    }
}

/// Get color for importance level
fn memory_importance_color(importance: &str) -> Color {
    match importance.to_lowercase().as_str() {
        "critical" => Color::Red,
        "high" => Color::LightRed,
        "normal" => Color::White,
        "low" => Color::DarkGray,
        _ => Color::White,
    }
}

/// Simple text wrapper
fn memory_wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width < 10 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current_line = String::new();
    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 > max_width {
            if !current_line.is_empty() {
                lines.push(current_line.trim().to_string());
                current_line = String::new();
            }
            if word.len() > max_width {
                // Char-safe chunking to avoid splitting multi-byte UTF-8
                let mut char_chunk = String::new();
                for ch in word.chars() {
                    if char_chunk.len() + ch.len_utf8() > max_width && !char_chunk.is_empty() {
                        lines.push(char_chunk);
                        char_chunk = String::new();
                    }
                    char_chunk.push(ch);
                }
                if !char_chunk.is_empty() {
                    lines.push(char_chunk);
                }
            } else {
                current_line = word.to_string();
            }
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn render_lsps(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();

    // Collect missing LSPs for installation guidance
    let missing_lsps: Vec<&str> = app
        .lsp_availability
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
            Constraint::Length(3),                                     // Header
            list_min,                                                  // LSP list
            Constraint::Length(if has_missing_lsps { 15 } else { 0 }), // Missing LSPs section
        ])
        .split(area);

    // Header block with control hints
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 🔌 Language Server Protocol (LSP) Status ")
        .title_style(Style::default().fg(theme.accent));

    let header_text = vec![Line::from(vec![
        Span::styled("Controls: ", Style::default().fg(theme.muted)),
        Span::styled("[s] Toggle ", Style::default().fg(theme.warning).bold()),
        Span::styled("| ", Style::default().fg(theme.muted)),
        Span::styled("[R] Restart ", Style::default().fg(theme.warning).bold()),
        Span::styled("| ", Style::default().fg(theme.muted)),
        Span::styled("[r] Refresh ", Style::default().fg(theme.warning).bold()),
        Span::styled("| ", Style::default().fg(theme.muted)),
        Span::styled("[l] Logs", Style::default().fg(theme.warning).bold()),
    ])];
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
                let short_title = session_title
                    .as_ref()
                    .map(|t| {
                        if t.chars().count() > 20 {
                            let truncated: String = t.chars().take(17).collect();
                            format!("{}...", truncated)
                        } else {
                            t.clone()
                        }
                    })
                    .unwrap_or_else(|| "Unknown".to_string());

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
            .highlight_style(
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .bold(),
            )
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
            Line::from(vec![Span::styled(
                "The following LSPs are not available on your system:",
                Style::default().fg(Color::Yellow).bold(),
            )]),
            Line::from(""),
        ];

        for lsp_name in &missing_lsps {
            let install_commands = crate::tabs::lsps::get_lsp_install_command(lsp_name);
            missing_lines.push(Line::from(vec![
                Span::styled(
                    format!("▸ {} ", lsp_name),
                    Style::default().fg(Color::Red).bold(),
                ),
                Span::styled("NOT FOUND", Style::default().fg(Color::Red).bold()),
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
                        Span::styled(format!("$ {}", cmd), Style::default().fg(Color::Cyan)),
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
            Constraint::Min(0),    // History
            Constraint::Length(3), // Progress / Status
            Constraint::Length(3), // Input Prompt
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
        Line::from(vec![Span::styled(
            " Maestro Analysis Engine v2.0 READY",
            Style::default().fg(Color::Green).bold(),
        )]),
        Line::from(vec![
            Span::styled(
                " Type '/phase1 <path>' to begin. ",
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                "(Press 'a' to enter Command Hub)",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(""),
    ];

    let examples = vec![
        Line::from(vec![Span::styled(
            " EXAMPLES:",
            Style::default().fg(Color::Yellow).bold(),
        )]),
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
        .title(if app.input_mode == InputMode::AnalysisPrompt {
            " ⌨️ Command (Esc/Enter to finish) > "
        } else {
            " Command > "
        })
        .title_style(Style::default().fg(Color::Cyan));

    let input_text = if app.input_mode == InputMode::AnalysisPrompt {
        format!("{}█", app.analysis_input)
    } else {
        app.analysis_input.clone()
    };

    let input = Paragraph::new(input_text).block(input_block).style(
        if app.input_mode == InputMode::AnalysisPrompt {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        },
    );
    frame.render_widget(input, chunks[2]);
}

fn render_sessions(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(if !app.preview_focused {
            BorderType::Double
        } else {
            BorderType::Rounded
        })
        .title(" 📁 Sessions & Groups ")
        .title_style(if !app.preview_focused {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_type(if app.preview_focused {
            BorderType::Double
        } else {
            BorderType::Rounded
        })
        .title(format!(
            " 🖥️ Preview {} ",
            if app.preview_focused { "[FOCUSED]" } else { "" }
        ))
        .title_style(if app.preview_focused {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default().fg(Color::DarkGray)
        });

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
                    items.push(ListItem::new(vec![Line::from(vec![
                        Span::styled(format!("  {} ", icon), Style::default().fg(Color::Yellow)),
                        Span::styled(&g.name, Style::default().bold().fg(Color::White)),
                        Span::styled(
                            if let Some(cat) = &g.category {
                                format!(" [{}]", cat)
                            } else {
                                "".to_string()
                            },
                            Style::default().fg(Color::Cyan),
                        ),
                        Span::styled(
                            format!(" ({})", g.path),
                            Style::default().fg(Color::DarkGray).italic(),
                        ),
                    ])]));
                }
                SessionEntry::Session(s) => {
                    let is_running =
                        s.status == leindex_core::memory::models::SessionStatus::Running;
                    let is_terminated =
                        s.status == leindex_core::memory::models::SessionStatus::Terminated;
                    let is_waiting =
                        s.status == leindex_core::memory::models::SessionStatus::Waiting;

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
                        Span::styled(
                            format!("  {}", branch),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(status_icon, Style::default().fg(status_color)),
                        Span::styled(&s.title, title_style),
                    ];

                    if s.status == leindex_core::memory::models::SessionStatus::Terminated {
                        line_spans.push(Span::styled(
                            " [KILLED]",
                            Style::default().fg(Color::Red).bold(),
                        ));
                    }

                    line_spans.push(Span::styled(
                        format!(" [{}]", s.tool.as_deref().unwrap_or("?")),
                        Style::default().fg(Color::DarkGray),
                    ));

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

        let list = List::new(items).block(list_block).highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 60))
                .fg(Color::White)
                .bold(),
        );
        frame.render_stateful_widget(list, chunks[0], &mut app.session_state);

        // Render Preview
        let mut preview_lines = Vec::new();

        if let Some(i) = app.session_state.selected() {
            if let Some(SessionEntry::Session(s)) = app.session_entries.get(i) {
                // Header (Replicating Go TUI)
                let status_icon = match s.status {
                    leindex_core::memory::models::SessionStatus::Running => "●",
                    leindex_core::memory::models::SessionStatus::Waiting => "◐",
                    _ => "○",
                };
                let status_color = match s.status {
                    leindex_core::memory::models::SessionStatus::Running => Color::Green,
                    leindex_core::memory::models::SessionStatus::Waiting => Color::Yellow,
                    leindex_core::memory::models::SessionStatus::Terminated => Color::Red,
                    _ => Color::DarkGray,
                };

                // Row 1: Icon Title (ID)
                preview_lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {} ", status_icon),
                        Style::default().fg(status_color).bold(),
                    ),
                    Span::styled(&s.title, Style::default().fg(Color::Cyan).bold()),
                    Span::styled(
                        format!(" ({})", s.session_id),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));

                // Row 2: Tool, Group, Activity
                let activity_str = "active now"; // Placeholder, replace with actual activity logic if available
                preview_lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {} ", s.tool.as_deref().unwrap_or("shell")),
                        Style::default().bg(theme.accent_alt).fg(Color::Black),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!(" {} ", s.group_path.as_deref().unwrap_or("Uncategorized")),
                        Style::default().bg(Color::Cyan).fg(Color::Black),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!(" ⏱ {}", activity_str),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));

                // Row 3: Path
                preview_lines.push(Line::from(vec![
                    Span::styled(" 📁 ", Style::default()),
                    Span::styled(&s.project_path, Style::default().fg(Color::DarkGray)),
                ]));

                // Row 4: Tool session IDs (best-effort capture)
                if let Some(ref metadata) = s.metadata {
                    if s.tool.as_deref() == Some("claude") {
                        if let Some(cid) =
                            metadata.get("claude_session_id").and_then(|v| v.as_str())
                        {
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
                        if let Some(gid) =
                            metadata.get("gemini_session_id").and_then(|v| v.as_str())
                        {
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
                        if let Some(cid) = metadata.get("codex_session_id").and_then(|v| v.as_str())
                        {
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
                        if let Some(oid) =
                            metadata.get("opencode_session_id").and_then(|v| v.as_str())
                        {
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

                    if let Some(mcps) = metadata.get("loaded_mcp_names").and_then(|v| v.as_array())
                    {
                        let mcp_names: Vec<String> = mcps
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                        if !mcp_names.is_empty() {
                            preview_lines.push(Line::from(vec![
                                Span::styled(" 🔌 MCPs: ", Style::default().fg(Color::Cyan)),
                                Span::styled(
                                    mcp_names.join(", "),
                                    Style::default().fg(Color::White),
                                ),
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
                preview_lines.push(Line::from(vec![Span::styled(
                    format!(" {} Output {} ", divider, divider),
                    Style::default().fg(Color::DarkGray),
                )]));
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

/// App-local wiring tests — guard against regressions in the startup path.
#[cfg(test)]
mod app_wiring_tests {
    use super::*;

    #[test]
    fn test_load_maestroclaw_workspace_dir_returns_non_empty() {
        let dir = load_maestroclaw_workspace_dir();
        assert!(
            !dir.as_os_str().is_empty(),
            "load_maestroclaw_workspace_dir() must return a non-empty path"
        );
        assert!(
            dir.to_string_lossy().contains("workspace"),
            "workspace_dir should contain 'workspace', got: {}",
            dir.display()
        );
    }

    #[test]
    fn test_load_maestroclaw_workspace_dir_creates_valid_pane() {
        let dir = load_maestroclaw_workspace_dir();
        let pane = crate::maesterclaw::MaestroClawPane::new(dir.clone());
        assert_eq!(
            pane.wizard.workspace_dir, dir,
            "MaestroClawPane wizard must use the same workspace_dir from config"
        );
    }

    /// Hermetic test: uses a temp home + real config fixture so the test never
    /// touches the developer's actual `~/.config/maestroclaw`.  Validates the
    /// full wiring path  `load_from_dir → workspace_dir → MaestroClawPane::new
    /// → wizard.workspace_dir`  with exact equality.
    #[test]
    fn test_workspace_dir_wiring_hermetic() {
        let tmp = tempfile::tempdir().expect("create temp dir");
        let config_dir = tmp.path().join(".config").join("maestroclaw");
        std::fs::create_dir_all(&config_dir).expect("create config dir");
        std::fs::create_dir_all(config_dir.join("workspace")).expect("create workspace dir");

        // Write a minimal config so load_from_dir actually parses a file.
        std::fs::write(
            config_dir.join("config.toml"),
            "primary_tool = \"claude\"\n",
        )
        .expect("write config");

        // Load config via the hermetic path (mirrors load_maestroclaw_workspace_dir
        // but without touching the real home directory).
        let config = maestro_claw::config::Config::load_from_dir(tmp.path().to_path_buf())
            .expect("load config from temp dir");
        let expected_ws = config.workspace_dir.clone();

        // Create the pane exactly as App::new does.
        let pane = crate::maesterclaw::MaestroClawPane::new(expected_ws.clone());

        assert_eq!(
            pane.wizard.workspace_dir, expected_ws,
            "MaestroClawPane wizard.workspace_dir must exactly match the loaded config workspace_dir"
        );
    }

    #[test]
    fn test_open_session_browser_and_select_switches_to_sessions_tab() {
        use crate::maesterclaw::MaestroClawAction;

        let mut app = App::new(None, None, None);

        // Seed the app with a test session so the browser has something to load.
        let session = leindex_core::memory::models::Session {
            id: 1,
            session_id: "sess-abc".to_string(),
            title: "My Test Session".to_string(),
            project_path: "/tmp/test".to_string(),
            group_path: None,
            sort_order: 0,
            parent_session_id: None,
            command: Some("echo hello".to_string()),
            tool: Some("claude".to_string()),
            status: leindex_core::memory::models::SessionStatus::Running,
            multiplexer_session: None,
            started_at: chrono::Utc::now(),
            last_accessed_at: Some(chrono::Utc::now()),
            ended_at: None,
            metadata: None,
        };
        app.sessions.push(session);

        // 1. OpenSessionBrowser should populate the browser and activate it.
        let handled = handle_maestroclaw_action(&mut app, MaestroClawAction::OpenSessionBrowser);
        assert!(handled);
        assert!(app.maestroclaw_pane.is_session_browser_active());

        // 2. SessionBrowserSelect should find the session and select it.
        let handled = handle_maestroclaw_action(&mut app, MaestroClawAction::SessionBrowserSelect);
        assert!(handled);
        assert!(app.maestroclaw_pane.selected_session.is_some());
        assert_eq!(app.status_message, "Selected session 'My Test Session' in MaestroClaw");
    }
}
