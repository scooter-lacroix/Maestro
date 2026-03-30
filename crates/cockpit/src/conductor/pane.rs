//! Conductor Pane - Ralph-style autonomous task execution
//!
//! This module provides the Conductor tab for the Cockpit TUI.
//! It integrates with the Maestro orchestrate engine to display
//! tracks, tasks, and execution progress in a command-center style UI.
//!
//! Inspired by Ralph TUI (https://ghuntley.com/ralph/)

use leindex_core::{
    multiplexer::TmuxMultiplexer,
    orchestrate::{
        model::{
            IterationLog, IterationStatus, LoopMode, SessionState, SessionStatus, Task, Track,
            TrackPlan, TrackStatus,
        },
        parser::{parse_plan_md, parse_tracks_md},
        setup::{detect_setup_status, AgentTool, SetupStatus},
    },
};
use maestro_pi_mono::agents::mapping::AgentRole;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use super::model::ConductorState;
use super::{
    input_modal::InputModal, memory_browser::MemoryBrowser, selector_modal::SelectorModal,
    TreeNodeId,
};
use crate::maestro_paths::MaestroProject;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandArgs {
    program: String,
    args: Vec<String>,
}

impl CommandArgs {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
        }
    }

    pub fn push_arg(&mut self, arg: impl Into<String>) {
        self.args.push(arg.into());
    }

    pub fn push_flag(&mut self, flag: &str) {
        self.args.push(flag.to_string());
    }

    /// Get the program name
    pub fn program(&self) -> &str {
        &self.program
    }

    /// Get the arguments
    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn spawn_detached(&self) -> std::io::Result<std::process::Child> {
        use std::process::{Command, Stdio};

        Command::new(&self.program)
            .args(&self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    }

    fn render_arg(arg: &str) -> String {
        if arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | ':'))
        {
            arg.to_string()
        } else {
            format!("'{}'", arg.replace('\'', "'\\''"))
        }
    }
}

impl std::fmt::Display for CommandArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::render_arg(&self.program))?;
        for arg in &self.args {
            write!(f, " {}", Self::render_arg(arg))?;
        }
        Ok(())
    }
}

/// Setup state for the conductor pane
#[derive(Debug, Clone)]
pub struct SetupState {
    /// Setup status (cached)
    pub status: Option<SetupStatus>,
    /// Show setup wizard
    pub show_setup_wizard: bool,
    /// Setup wizard step (0 = welcome, 1 = tool selection, 2 = confirm)
    pub wizard_step: usize,
    /// Selected tool in wizard
    pub selected_tool: Option<AgentTool>,
}

impl Default for SetupState {
    fn default() -> Self {
        Self {
            status: None,
            show_setup_wizard: false,
            wizard_step: 0,
            selected_tool: None,
        }
    }
}

/// State for the Conductor pane
pub struct ConductorPane {
    /// Ralph-style centralized state
    pub state: ConductorState,
    /// Selected item index in the flattened tree
    pub selected_index: usize,
    /// Currently selected details view mode
    pub details_mode: crate::conductor::model::DetailsViewMode,
    /// List of tracks loaded from tracks.md
    pub tracks: Vec<Track>,
    /// Task tree expansion state (task_id -> expanded)
    pub expanded_tasks: std::collections::HashSet<String>,
    /// Current session status
    pub session_status: SessionStatus,
    /// Current loop mode (planning/building)
    pub loop_mode: LoopMode,
    /// Current iteration number
    pub current_iteration: u64,
    /// Output from the current iteration
    pub iteration_output: Vec<String>,
    /// Output scroll offset
    pub output_scroll: u16,
    /// Whether the output panel is focused
    pub output_focused: bool,
    /// Tracks directory path
    pub tracks_dir: PathBuf,
    /// Error message to display
    pub error_message: Option<String>,
    /// Cached plan for the selected track (to avoid disk I/O in render loop)
    cached_plan: Option<TrackPlan>,
    /// Track index for which the plan is cached
    cached_plan_track_index: Option<usize>,
    /// Cached plan file mtimes (track_id -> mtime)
    plan_mtime_cache: std::collections::HashMap<String, std::time::SystemTime>,
    /// Tracks discovered from ~/.maestro/orchestrate (external sessions)
    external_track_ids: std::collections::HashSet<String>,
    /// Last time external sessions were scanned
    last_external_scan: std::time::Instant,
    /// Last known tracks.md modification time
    last_tracks_mtime: Option<std::time::SystemTime>,
    /// Setup state
    pub setup: SetupState,
    /// Whether to show the dashboard overlay
    pub show_dashboard: bool,
    /// Current Maestro project info
    pub current_project: Option<MaestroProject>,
    /// Steering input modal
    pub steering_modal: InputModal,
    /// Iteration-limit input modal
    pub iter_modal: InputModal,
    /// Generic selector modal
    pub selector_modal: SelectorModal,
    /// Memory browser overlay
    pub memory_browser: MemoryBrowser,
    /// Active cancellation token for background execution
    pub cancellation_token: Option<Arc<CancellationToken>>,
    /// Selected row in the iteration history list
    pub selected_iteration_log: usize,
    /// Whether the iteration history list is focused
    pub iteration_history_focused: bool,
    /// Whether the iteration detail popup is visible
    pub show_iteration_popup: bool,
}

impl Default for ConductorPane {
    fn default() -> Self {
        Self {
            state: {
                let mut state = ConductorState::default();
                state.omp_available = crate::omp::is_omp_available();
                state
            },
            selected_index: 0,
            details_mode: crate::conductor::model::DetailsViewMode::Details,
            tracks: Vec::new(),
            expanded_tasks: std::collections::HashSet::new(),
            session_status: SessionStatus::Idle,
            loop_mode: LoopMode::Building,
            current_iteration: 0,
            iteration_output: Vec::new(),
            output_scroll: 0,
            output_focused: false,
            tracks_dir: PathBuf::from("."),
            error_message: None,
            cached_plan: None,
            cached_plan_track_index: None,
            plan_mtime_cache: std::collections::HashMap::new(),
            external_track_ids: std::collections::HashSet::new(),
            last_external_scan: std::time::Instant::now(),
            last_tracks_mtime: None,
            setup: Default::default(),
            show_dashboard: false,
            current_project: None,
            steering_modal: InputModal::new(
                "Steering Message",
                "Enter guidance for the next iteration:",
            ),
            iter_modal: InputModal::new("Max Iterations", "Enter max iterations (0 = unlimited):"),
            selector_modal: SelectorModal {
                title: String::new(),
                items: Vec::new(),
                selected: 0,
                visible: false,
            },
            memory_browser: MemoryBrowser::default(),
            cancellation_token: None,
            selected_iteration_log: 0,
            iteration_history_focused: false,
            show_iteration_popup: false,
        }
    }
}

impl ConductorPane {
    pub fn push_runtime_log(
        &mut self,
        level: super::model::RuntimeLogLevel,
        iteration: Option<u64>,
        task_id: Option<String>,
        summary: impl Into<String>,
        details: Option<String>,
    ) {
        self.state.runtime_logs.push(super::model::RuntimeLogEntry {
            timestamp: chrono::Utc::now(),
            iteration,
            task_id,
            summary: summary.into(),
            details,
            level,
        });
        if self.state.runtime_logs.len() > 200 {
            self.state.runtime_logs.remove(0);
        }
    }

    pub fn move_iteration_selection(&mut self, delta: i32) {
        if self.state.iteration_logs.is_empty() {
            self.selected_iteration_log = 0;
            return;
        }

        let max = self.state.iteration_logs.len().saturating_sub(1);
        if delta > 0 {
            self.selected_iteration_log = (self.selected_iteration_log + delta as usize).min(max);
        } else {
            self.selected_iteration_log = self
                .selected_iteration_log
                .saturating_sub(delta.unsigned_abs() as usize);
        }
    }

    pub fn selected_iteration_log(&self) -> Option<&IterationLog> {
        if self.state.iteration_logs.is_empty() {
            return None;
        }
        let reversed_index = self
            .state
            .iteration_logs
            .len()
            .saturating_sub(1)
            .saturating_sub(self.selected_iteration_log);
        self.state.iteration_logs.get(reversed_index)
    }

    fn load_tracks_from_open_projects(
        &self,
        tracks: &mut Vec<Track>,
        external_ids: &mut std::collections::HashSet<String>,
    ) {
        let discovered_projects = crate::maestro_paths::discover_all_projects();
        let current_tracks_dir = self
            .tracks_dir
            .canonicalize()
            .unwrap_or_else(|_| self.tracks_dir.clone());

        for project in discovered_projects {
            let project_tracks_dir = project
                .tracks_dir
                .canonicalize()
                .unwrap_or_else(|_| project.tracks_dir.clone());

            if project_tracks_dir == current_tracks_dir {
                continue;
            }

            let tracks_path = project.tracks_dir.join("tracks.md");
            let Ok(project_tracks) = parse_tracks_md(&tracks_path) else {
                continue;
            };

            for mut track in project_tracks {
                if tracks.iter().any(|existing| existing.id == track.id) {
                    continue;
                }

                track.description = format!("[{}] {}", project.name(), track.description);
                external_ids.insert(track.id.clone());
                tracks.push(track);
            }
        }
    }

    fn selected_track_dir(&self, track_idx: usize) -> PathBuf {
        self.tracks
            .get(track_idx)
            .and_then(|track| track.link_path.parent().map(|path| path.to_path_buf()))
            .unwrap_or_else(|| self.tracks_dir.clone())
    }

    /// Create a new conductor pane
    pub fn new(tracks_dir: PathBuf) -> Self {
        let mut pane = Self {
            tracks_dir,
            ..Default::default()
        };
        pane.load_tracks().ok();
        pane.check_setup_status();
        pane
    }

    /// Create a conductor pane with auto-resolved tracks directory.
    /// Uses maestro_paths to discover the tracks directory from current working dir
    /// and active tmux panes.
    pub fn auto_discover() -> Self {
        let projects = crate::maestro_paths::discover_all_projects();

        // Prefer the project matching current working directory, or first available
        let project = if let Ok(cwd) = std::env::current_dir() {
            projects
                .iter()
                .find(|p| cwd.starts_with(&p.root_dir))
                .or_else(|| projects.first())
                .cloned()
        } else {
            projects.first().cloned()
        };

        let tracks_dir = project
            .as_ref()
            .map(|p| p.tracks_dir.clone())
            .unwrap_or_else(|| PathBuf::from("."));
        let mut pane = Self::new(tracks_dir);
        pane.current_project = project;
        pane
    }

    /// Load tracks from tracks.md
    pub fn load_tracks(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let tracks_path = self.tracks_dir.join("tracks.md");

        let mut loaded_tracks = if tracks_path.exists() {
            match parse_tracks_md(&tracks_path) {
                Ok(tracks) => tracks,
                Err(e) => {
                    self.error_message = Some(format!("Error loading tracks.md: {}", e));
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        // Discover external sessions from ~/.maestro/orchestrate
        let mut external_ids = std::collections::HashSet::new();
        self.load_tracks_from_open_projects(&mut loaded_tracks, &mut external_ids);
        self.discover_external_sessions(&mut loaded_tracks, &mut external_ids);

        self.tracks = loaded_tracks;
        self.external_track_ids = external_ids;
        self.last_tracks_mtime = tracks_path.metadata().and_then(|m| m.modified()).ok();
        self.last_external_scan = std::time::Instant::now();
        self.rebuild_normalized_tree()
            .map_err(|err| format!("failed to build conductor tree: {err}"))?;
        self.sync_selection_from_legacy();
        Ok(())
    }

    fn discover_external_sessions(
        &self,
        tracks: &mut Vec<Track>,
        external_ids: &mut std::collections::HashSet<String>,
    ) {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let orchestrate_base = PathBuf::from(home).join(".maestro").join("orchestrate");

        if let Ok(entries) = std::fs::read_dir(orchestrate_base) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let track_id = entry.file_name().to_string_lossy().to_string();

                        // If not already in tracks, add it as an external track
                        if !tracks.iter().any(|t| t.id == track_id) {
                            let session_path = entry.path().join("session.json");
                            if session_path.exists() {
                                if let Ok(content) = std::fs::read_to_string(&session_path) {
                                    if let Ok(session) =
                                        serde_json::from_str::<SessionState>(&content)
                                    {
                                        // Skip completed/failed sessions to avoid stale phantom entries.
                                        if matches!(
                                            session.status,
                                            SessionStatus::Completed
                                                | SessionStatus::Failed
                                                | SessionStatus::Interrupted
                                                | SessionStatus::Idle
                                        ) {
                                            continue;
                                        }

                                        let track_status = match session.status {
                                            SessionStatus::Running | SessionStatus::Paused => {
                                                TrackStatus::InProgress
                                            }
                                            SessionStatus::Completed => TrackStatus::Completed,
                                            _ => TrackStatus::Pending,
                                        };

                                        // Create a placeholder Track for this external session
                                        let mut track = Track {
                                            id: track_id.clone(),
                                            description: "CLI/External Session".to_string(),
                                            status: track_status,
                                            link_path: entry.path(), // Use orchestrate dir as link path for now
                                            metadata: None,
                                            plan: None,
                                        };

                                        // Synthesize a virtual plan from iterations if possible
                                        if let Ok(logs) = self.load_iterations_for_track(&track_id)
                                        {
                                            if !logs.is_empty() {
                                                track.plan =
                                                    Some(self.synthesize_plan_from_logs(
                                                        &track_id, &logs,
                                                    ));
                                            }
                                        }

                                        tracks.push(track);
                                        external_ids.insert(track_id.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn load_iterations_for_track(
        &self,
        track_id: &str,
    ) -> Result<Vec<IterationLog>, Box<dyn std::error::Error>> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let log_path = PathBuf::from(home)
            .join(".maestro")
            .join("orchestrate")
            .join(track_id)
            .join("iterations.jsonl");

        if !log_path.exists() {
            return Ok(Vec::new());
        }

        let file = std::fs::File::open(log_path)?;
        let reader = std::io::BufReader::new(file);
        let mut logs = Vec::new();
        for line in std::io::BufRead::lines(reader).flatten() {
            if let Ok(log) = serde_json::from_str::<IterationLog>(&line) {
                logs.push(log);
            }
        }
        Ok(logs)
    }

    fn synthesize_plan_from_logs(&self, track_id: &str, logs: &[IterationLog]) -> TrackPlan {
        use std::collections::HashSet;
        let mut seen_tasks = HashSet::new();
        let mut tasks = Vec::new();

        for log in logs {
            if seen_tasks.insert(&log.task_id) {
                tasks.push(Task {
                    id: log.task_id.clone(),
                    title: log.task_id.clone(), // Use ID as title for virtual tasks
                    status: match log.status {
                        IterationStatus::Completed => TrackStatus::Completed,
                        IterationStatus::Running => TrackStatus::InProgress,
                        _ => TrackStatus::Pending,
                    },
                    dependencies: Vec::new(),
                    description: "Synthesized from iteration history".to_string(),
                    subtasks: Vec::new(),
                    notes: None,
                    line_number: 0,
                });
            } else if let Some(t) = tasks.iter_mut().find(|t| t.id == log.task_id) {
                // Update status to latest
                t.status = match log.status {
                    IterationStatus::Completed => TrackStatus::Completed,
                    IterationStatus::Running => TrackStatus::InProgress,
                    _ => t.status,
                };
            }
        }

        TrackPlan {
            track_id: track_id.to_string(),
            tasks,
            phases: Vec::new(),
        }
    }

    /// Reload tracks (call when tracks_dir changes or to refresh)
    pub fn reload(&mut self) {
        self.invalidate_plan_cache();
        self.plan_mtime_cache.clear();
        self.external_track_ids.clear();
        self.last_tracks_mtime = None;
        self.tracks.clear();
        self.selected_index = 0;
        let _ = self.load_tracks();
        // Rebuild normalized tree after loading tracks
        let _ = self.rebuild_normalized_tree();
    }

    /// Set the tracks directory and reload
    pub fn set_tracks_dir(&mut self, tracks_dir: PathBuf) {
        self.tracks_dir = tracks_dir;
        self.reload();
    }

    /// Sync active tmux sessions from the Sessions tab as tracks
    /// This adds running tmux sessions that aren't already tracked
    pub fn sync_sessions_as_tracks(&mut self, sessions: &[leindex_core::memory::models::Session]) {
        use leindex_core::memory::models::SessionStatus;

        for session in sessions {
            // Skip if already tracked
            if self.tracks.iter().any(|t| t.id == session.session_id) {
                continue;
            }

            // Only add RUNNING sessions - filter out Terminated, Waiting, etc.
            // Use direct enum comparison instead of string matching
            if session.status != SessionStatus::Running {
                continue;
            }

            let track = Track {
                id: session.session_id.clone(),
                description: format!("Tmux Session: {}", session.title),
                status: TrackStatus::InProgress,
                link_path: PathBuf::from(&session.project_path),
                metadata: None,
                plan: None,
            };

            self.tracks.push(track);
        }
    }

    /// Switch projects automatically based on the active tmux pane.
    /// Returns true if a project switch occurred.
    pub fn refresh_project_from_tmux(&mut self) -> bool {
        if std::env::var("TMUX").is_err() {
            return false;
        }

        let mux = TmuxMultiplexer::new();
        let Some(active_path) = mux.get_active_pane_path().ok().flatten() else {
            return false;
        };

        let Some(project) =
            crate::maestro_paths::resolve_maestro_project(Some(Path::new(&active_path)))
        else {
            return false;
        };

        let should_switch = self
            .current_project
            .as_ref()
            .map(|p| p.root_dir != project.root_dir)
            .unwrap_or(true);

        if should_switch {
            self.switch_project(project);
        }

        should_switch
    }

    /// Refresh tracks if tracks.md or external sessions have changed.
    pub fn refresh_tracks_if_needed(&mut self) {
        use std::time::Duration;

        let tracks_path = self.tracks_dir.join("tracks.md");
        let tracks_mtime = tracks_path.metadata().and_then(|m| m.modified()).ok();
        let tracks_changed = match (tracks_mtime, self.last_tracks_mtime) {
            (Some(new), Some(old)) => new > old,
            (Some(_), None) => true,
            (None, Some(_)) => true,
            (None, None) => false,
        };

        let external_scan_due = self.last_external_scan.elapsed() >= Duration::from_secs(2);
        if external_scan_due {
            if self.refresh_project_from_tmux() {
                return;
            }
        }

        if !tracks_changed && !external_scan_due {
            return;
        }

        let mut loaded_tracks = if tracks_path.exists() {
            match parse_tracks_md(&tracks_path) {
                Ok(tracks) => tracks,
                Err(e) => {
                    self.error_message = Some(format!("Error loading tracks.md: {}", e));
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        let mut external_ids = std::collections::HashSet::new();
        self.load_tracks_from_open_projects(&mut loaded_tracks, &mut external_ids);
        self.discover_external_sessions(&mut loaded_tracks, &mut external_ids);

        self.tracks = loaded_tracks;

        // Update external_track_ids to only include tracks that survived filtering
        self.external_track_ids = self
            .tracks
            .iter()
            .filter(|t| external_ids.contains(&t.id))
            .map(|t| t.id.clone())
            .collect();

        self.last_tracks_mtime = tracks_mtime;
        self.last_external_scan = std::time::Instant::now();
        self.invalidate_plan_cache();

        // Get current selected item before adjusting selection
        let current_selection = self.get_current_selection_id();

        // Only change selection if the previously selected item no longer exists
        // This prevents focus jumping on every refresh
        let items = self.get_selectable_items();
        if let Some(current_id) = current_selection {
            let current_still_exists = items.iter().any(|item| match item {
                crate::conductor::model::SelectableItem::Track { id, .. } => id == &current_id,
                crate::conductor::model::SelectableItem::Task { id, .. } => id == &current_id,
            });

            if !current_still_exists {
                // Current selection was removed, clamp to valid range
                if items.is_empty() {
                    self.selected_index = 0;
                } else if self.selected_index >= items.len() {
                    self.selected_index = items.len() - 1;
                }
            }
            // If current selection still exists, don't change selected_index
        } else {
            // No previous selection, ensure valid range
            if items.is_empty() {
                self.selected_index = 0;
            } else if self.selected_index >= items.len() {
                self.selected_index = items.len() - 1;
            }
        }

        let mut valid_ids: std::collections::HashSet<String> =
            self.tracks.iter().map(|t| t.id.clone()).collect();
        for idx in 0..self.tracks.len() {
            if let Ok(Some(plan)) = self.load_track_plan_internal(idx) {
                Self::collect_valid_task_ids(&plan.tasks, &mut valid_ids);
            }
        }
        self.expanded_tasks.retain(|id| valid_ids.contains(id));
        self.state
            .track_runtime_statuses
            .retain(|id, _| valid_ids.contains(id));

        // Rebuild normalized tree with updated data
        let _ = self.rebuild_normalized_tree();
    }

    /// Open the project selector
    pub fn open_project_selector(&mut self) {
        self.state.available_projects = crate::maestro_paths::discover_all_projects();
        self.state.show_project_selector = true;
        self.state.selected_project_index = 0;
    }

    /// Switch to a different Maestro project
    pub fn switch_project(&mut self, project: MaestroProject) {
        self.tracks_dir = project.tracks_dir.clone();
        self.current_project = Some(project);
        self.reload();
        // Reset state that belongs to the old project
        self.state.current_track = None;
        self.state.current_task = None;
        self.state.session_id = None;
        self.state.subagents.clear();
        self.state.tasks_completed = 0;
        self.state.total_tasks = 0;
        self.state.show_project_selector = false;
        self.error_message = None;
    }

    /// Invalidate the cached plan (call when plan.md changes externally)
    pub fn invalidate_plan_cache(&mut self) {
        self.cached_plan = None;
        self.cached_plan_track_index = None;
        self.plan_mtime_cache.clear();
    }

    /// Get the flat list of selectable items for the tree
    pub fn get_selectable_items(&mut self) -> Vec<crate::conductor::model::SelectableItem> {
        let mut items = Vec::new();

        // We need to work with a clone of tracks to avoid borrowing issues while we might need to load plans
        let tracks_clone = self.tracks.clone();

        for (idx, track) in tracks_clone.iter().enumerate() {
            let is_master = track.id.contains("master") || track.is_master();
            // Check external_track_ids set for reliable identification
            let is_external = self.external_track_ids.contains(&track.id);
            let is_expanded = self.expanded_tasks.contains(&track.id);

            items.push(crate::conductor::model::SelectableItem::Track {
                index: idx,
                id: track.id.clone(),
                is_master,
                is_external,
                is_expanded,
            });

            // If the track is expanded, show its tasks.
            // A track is expanded if it's in expanded_tasks.
            if self.expanded_tasks.contains(&track.id) {
                if let Ok(Some(plan)) = self.load_track_plan_internal(idx) {
                    for task in &plan.tasks {
                        self.add_tasks_to_selectable_items(task, 1, &mut items);
                    }
                }
            }
        }
        items
    }

    fn add_tasks_to_selectable_items(
        &self,
        task: &Task,
        depth: usize,
        items: &mut Vec<crate::conductor::model::SelectableItem>,
    ) {
        let is_expanded = self.expanded_tasks.contains(&task.id);
        let has_children = !task.subtasks.is_empty();
        let completed_tasks = self.collect_completed_tasks();
        let dependency_statuses = task
            .dependencies
            .iter()
            .map(
                |dep| match completed_tasks.get(&dep.task_id).copied().unwrap_or(false) {
                    true => crate::conductor::model::DependencyStatus::Completed,
                    false
                        if dep.dependency_type
                            == leindex_core::orchestrate::model::TaskDependencyType::Hard =>
                    {
                        crate::conductor::model::DependencyStatus::Blocked
                    }
                    false => crate::conductor::model::DependencyStatus::Pending,
                },
            )
            .collect();

        items.push(crate::conductor::model::SelectableItem::Task {
            id: task.id.clone(),
            title: task.title.clone(),
            depth,
            status: task.status,
            has_children,
            is_expanded,
            description: task.description.clone(),
            notes: task.notes.clone().unwrap_or_default(),
            is_blocked: task.is_blocked(&completed_tasks),
            is_actionable: task.is_actionable(&completed_tasks),
            dependencies: task.dependencies.clone(),
            dependency_statuses,
        });

        if is_expanded {
            for subtask in &task.subtasks {
                self.add_tasks_to_selectable_items(subtask, depth + 1, items);
            }
        }
    }

    fn collect_completed_tasks(&self) -> std::collections::HashMap<String, bool> {
        fn visit(task: &Task, map: &mut std::collections::HashMap<String, bool>) {
            map.insert(task.id.clone(), task.status == TrackStatus::Completed);
            for subtask in &task.subtasks {
                visit(subtask, map);
            }
        }

        let mut completed = std::collections::HashMap::new();
        if let Some(plan) = self.cached_plan.as_ref() {
            for task in &plan.tasks {
                visit(task, &mut completed);
            }
        }
        completed
    }

    fn collect_valid_task_ids(tasks: &[Task], valid_ids: &mut std::collections::HashSet<String>) {
        for task in tasks {
            valid_ids.insert(task.id.clone());
            Self::collect_valid_task_ids(&task.subtasks, valid_ids);
        }
    }

    /// Internal helper to load plan without borrowing self.tracks
    fn load_track_plan_internal(
        &mut self,
        track_idx: usize,
    ) -> Result<Option<TrackPlan>, Box<dyn std::error::Error>> {
        if track_idx >= self.tracks.len() {
            return Ok(None);
        }

        let track = &self.tracks[track_idx];
        let plan_path = track.link_path.join("plan.md");
        let plan_mtime = plan_path.metadata().and_then(|m| m.modified()).ok();

        if self.cached_plan_track_index == Some(track_idx) {
            if let Some(ref plan) = self.cached_plan {
                let cached_mtime = self.plan_mtime_cache.get(&track.id).copied();
                if cached_mtime.is_some() && cached_mtime == plan_mtime {
                    return Ok(Some(plan.clone()));
                }
            }
        }

        // If track already has a plan (e.g. synthesized), use it
        if let Some(ref plan) = track.plan {
            self.cached_plan = Some(plan.clone());
            self.cached_plan_track_index = Some(track_idx);
            if let Some(mtime) = plan_mtime {
                self.plan_mtime_cache.insert(track.id.clone(), mtime);
            }
            return Ok(Some(plan.clone()));
        }

        if !plan_path.exists() {
            return Ok(None);
        }

        let plan = parse_plan_md(&plan_path)?;
        self.cached_plan = Some(plan.clone());
        self.cached_plan_track_index = Some(track_idx);
        if let Some(mtime) = plan_mtime {
            self.plan_mtime_cache.insert(track.id.clone(), mtime);
        }
        Ok(Some(plan))
    }

    /// Toggle expansion of the selected task or track
    ///
    /// This updates both the legacy expanded_tasks HashSet and the normalized tree
    /// for backward compatibility during the transition.
    pub fn toggle_task_expansion(&mut self, id: &str) {
        // Update legacy expansion state
        if self.expanded_tasks.contains(id) {
            self.expanded_tasks.remove(id);
        } else {
            self.expanded_tasks.insert(id.to_string());
        }

        // Also update normalized tree if the node exists
        // Try different node types to find the right one
        let track_node_id = TreeNodeId::track(id);
        let task_node_id = TreeNodeId::task(id);
        let session_node_id = TreeNodeId::session(id);

        if self
            .state
            .normalized_tree
            .nodes
            .contains_key(&track_node_id)
        {
            self.state.normalized_tree.toggle_expanded(&track_node_id);
        } else if self.state.normalized_tree.nodes.contains_key(&task_node_id) {
            self.state.normalized_tree.toggle_expanded(&task_node_id);
        } else if self
            .state
            .normalized_tree
            .nodes
            .contains_key(&session_node_id)
        {
            self.state.normalized_tree.toggle_expanded(&session_node_id);
        }
    }

    /// Check if a task or track is expanded
    pub fn is_task_expanded(&self, id: &str) -> bool {
        self.expanded_tasks.contains(id)
    }

    /// Move selection up or down the flat list
    pub fn move_selection(&mut self, delta: i32) {
        let items = self.get_selectable_items();
        if items.is_empty() {
            self.selected_index = 0;
            self.state.normalized_tree.selected_node = None;
            return;
        }

        if delta > 0 {
            self.selected_index = self.selected_index.saturating_add(delta as usize);
        } else {
            self.selected_index = self.selected_index.saturating_sub(delta.abs() as usize);
        }

        if self.selected_index >= items.len() {
            self.selected_index = items.len() - 1;
        }

        self.sync_selection_from_legacy();
    }

    /// Get currently selected track index
    pub fn get_selected_track_index(&mut self) -> Option<usize> {
        let items = self.get_selectable_items();
        if items.is_empty() {
            return None;
        }

        let idx = self.selected_index.min(items.len() - 1);
        for i in (0..=idx).rev() {
            if let crate::conductor::model::SelectableItem::Track { index, .. } = items[i] {
                return Some(index);
            }
        }
        None
    }

    /// Get the ID of the currently selected item (track or task)
    /// Used to preserve selection after refreshing tracks
    pub fn get_current_selection_id(&mut self) -> Option<String> {
        let items = self.get_selectable_items();
        if items.is_empty() {
            return None;
        }

        let idx = self.selected_index.min(items.len() - 1);
        match &items[idx] {
            crate::conductor::model::SelectableItem::Track { id, .. } => Some(id.clone()),
            crate::conductor::model::SelectableItem::Task { id, .. } => Some(id.clone()),
        }
    }

    /// Select next track
    pub fn next_track(&mut self) {
        if self.tracks.is_empty() {
            return;
        }

        let current_track_idx = self.get_selected_track_index().unwrap_or(0);
        let next_idx = (current_track_idx + 1) % self.tracks.len();

        // Find the index of the next track in the flattened list
        let items = self.get_selectable_items();
        for (i, item) in items.iter().enumerate() {
            if let crate::conductor::model::SelectableItem::Track { index, .. } = item {
                if *index == next_idx {
                    self.selected_index = i;
                    self.sync_selection_from_legacy();
                    break;
                }
            }
        }
    }

    /// Select previous track
    pub fn prev_track(&mut self) {
        if self.tracks.is_empty() {
            return;
        }

        let current_track_idx = self.get_selected_track_index().unwrap_or(0);
        let prev_idx = if current_track_idx == 0 {
            self.tracks.len() - 1
        } else {
            current_track_idx - 1
        };

        // Find the index of the previous track in the flattened list
        let items = self.get_selectable_items();
        for (i, item) in items.iter().enumerate() {
            if let crate::conductor::model::SelectableItem::Track { index, .. } = item {
                if *index == prev_idx {
                    self.selected_index = i;
                    self.sync_selection_from_legacy();
                    break;
                }
            }
        }
    }

    /// Add output line
    pub fn add_output(&mut self, line: String) {
        self.iteration_output.push(line);
        // Keep only last 1000 lines
        if self.iteration_output.len() > 1000 {
            self.iteration_output = self
                .iteration_output
                .iter()
                .skip(self.iteration_output.len() - 1000)
                .cloned()
                .collect();
        }
    }

    /// Scroll output up
    pub fn scroll_output_up(&mut self) {
        self.output_scroll = self.output_scroll.saturating_add(1);
    }

    /// Scroll output down
    pub fn scroll_output_down(&mut self) {
        self.output_scroll = self.output_scroll.saturating_sub(1);
    }

    /// Clear output
    pub fn clear_output(&mut self) {
        self.iteration_output.clear();
        self.state.runtime_logs.clear();
        self.output_scroll = 0;
    }

    /// Check setup status and cache the result
    pub fn check_setup_status(&mut self) {
        if let Ok(status) = detect_setup_status(&self.tracks_dir) {
            // Check if minimally configured before moving
            let needs_wizard = !status.is_minimally_configured() && !self.setup.show_setup_wizard;
            self.setup.status = Some(status);

            // Auto-show setup wizard if not minimally configured
            if needs_wizard {
                self.setup.show_setup_wizard = true;
                self.setup.wizard_step = 0;
            }
        }
    }

    /// Get the cached setup status, checking if not cached
    pub fn get_setup_status(&mut self) -> Option<&SetupStatus> {
        if self.setup.status.is_none() {
            self.check_setup_status();
        }
        self.setup.status.as_ref()
    }

    /// Dismiss the setup wizard
    pub fn dismiss_setup_wizard(&mut self) {
        self.setup.show_setup_wizard = false;
        self.setup.wizard_step = 0;
    }

    /// Advance the setup wizard
    pub fn advance_setup_wizard(&mut self) {
        self.setup.wizard_step += 1;
    }

    /// Select a tool in the setup wizard
    pub fn select_setup_tool(&mut self, tool: AgentTool) {
        self.setup.selected_tool = Some(tool);
    }

    /// Get the recommended start command for the current track
    pub fn get_start_command(
        &mut self,
        tool: Option<&str>,
        dangerous: bool,
        sandbox: bool,
    ) -> Option<CommandArgs> {
        let track_idx = match self.get_selected_track_index() {
            Some(idx) => idx,
            None => return None,
        };

        let track_id = &self.tracks[track_idx].id;

        // Use setup wizard selected tool, or provided tool, or default to claude
        let tool = if let Some(setup_tool) = &self.setup.selected_tool {
            setup_tool.as_str()
        } else {
            tool.unwrap_or("claude")
        };

        let mode_str = match self.loop_mode {
            LoopMode::Planning => "planning",
            LoopMode::Building => "building",
        };

        let mut cmd = CommandArgs::new("maestro");
        cmd.push_arg("orchestrate");
        cmd.push_arg("start");
        cmd.push_arg(track_id);
        cmd.push_arg("--mode");
        cmd.push_arg(mode_str);
        cmd.push_arg("--tool");
        cmd.push_arg(tool);

        if dangerous {
            cmd.push_flag("--dangerous");
        }

        if sandbox {
            cmd.push_flag("--sandbox");
        }

        cmd.push_arg("--tracks-dir");
        cmd.push_arg(self.selected_track_dir(track_idx).display().to_string());

        Some(cmd)
    }

    /// Get the recommended pause command for the current track
    pub fn get_pause_command(&mut self) -> Option<CommandArgs> {
        let track_idx = match self.get_selected_track_index() {
            Some(idx) => idx,
            None => return None,
        };

        let track_id = &self.tracks[track_idx].id;
        let mut cmd = CommandArgs::new("maestro");
        cmd.push_arg("orchestrate");
        cmd.push_arg("pause");
        cmd.push_arg(track_id);
        cmd.push_arg("--tracks-dir");
        cmd.push_arg(self.selected_track_dir(track_idx).display().to_string());
        Some(cmd)
    }

    /// Get the recommended resume command for the current track
    pub fn get_resume_command(&mut self) -> Option<CommandArgs> {
        let track_idx = match self.get_selected_track_index() {
            Some(idx) => idx,
            None => return None,
        };

        let track_id = &self.tracks[track_idx].id;
        let mut cmd = CommandArgs::new("maestro");
        cmd.push_arg("orchestrate");
        cmd.push_arg("resume");
        cmd.push_arg(track_id);
        cmd.push_arg("--tracks-dir");
        cmd.push_arg(self.selected_track_dir(track_idx).display().to_string());
        Some(cmd)
    }

    /// Get the recommended status command for the current track
    pub fn get_status_command(&mut self) -> CommandArgs {
        let mut cmd = CommandArgs::new("maestro");
        cmd.push_arg("orchestrate");
        cmd.push_arg("status");
        if let Some(track_idx) = self.get_selected_track_index() {
            cmd.push_arg(&self.tracks[track_idx].id);
        }
        cmd.push_arg("--tracks-dir");
        cmd.push_arg(self.tracks_dir.display().to_string());
        cmd
    }

    /// Get the command to create a new track
    pub fn get_new_track_command(&self) -> String {
        "maestro newTrack".to_string()
    }

    pub fn poll_observed_sessions_sync(&mut self) {
        let _ = self.load_tracks();
    }

    pub fn sync_orchestrate_sessions_blocking(&mut self) {
        let _ = self.load_tracks();
        self.state.current_track = self
            .get_selected_track_index()
            .and_then(|idx| self.tracks.get(idx))
            .map(|track| track.id.clone());
    }

    pub fn cycle_agent_role(&mut self) -> AgentRole {
        let current = match self.state.selected_agent_role.as_deref() {
            Some("Scout") => Some(AgentRole::Scout),
            Some("Architect") => Some(AgentRole::Architect),
            Some("Critic") => Some(AgentRole::Critic),
            Some("Kraken") => Some(AgentRole::Kraken),
            _ => None,
        };
        let next = super::agent_executor::role_utils::cycle_role(&current);
        self.state.selected_agent_role =
            Some(super::agent_executor::role_utils::role_display_name(&next).to_string());
        next
    }

    /// Rebuild the normalized conductor tree from current data sources
    ///
    /// This uses the TreeBuilder to create a normalized tree that merges
    /// tracks, sessions, and external sessions into a single hierarchy.
    pub fn rebuild_normalized_tree(&mut self) -> Result<(), String> {
        use super::tree_builder::{TreeBuilder, TreeBuilderConfig};

        // Load external sessions from ~/.maestro/orchestrate
        let mut external_sessions = Vec::new();
        self.collect_external_sessions_for_tree(&mut external_sessions);
        let runtime_sessions = self.collect_runtime_sessions();

        // Load metadata for all tracks (tracks already have their plans from parsing)
        let mut metadata = std::collections::HashMap::new();
        let mut plans = std::collections::HashMap::new();

        for idx in 0..self.tracks.len() {
            let track_id = self.tracks[idx].id.clone();
            // Try to load metadata
            if let Ok(meta_content) =
                std::fs::read_to_string(self.tracks_dir.join(&track_id).join("metadata.json"))
            {
                if let Ok(track_meta) = serde_json::from_str::<
                    leindex_core::orchestrate::model::TrackMetadata,
                >(&meta_content)
                {
                    metadata.insert(track_id.clone(), track_meta);
                }
            }

            if let Ok(Some(plan)) = self.load_track_plan_internal(idx) {
                if let Some(track) = self.tracks.get_mut(idx) {
                    track.plan = Some(plan.clone());
                }
                plans.insert(track_id, plan);
            }
        }

        // Build tree using builder pattern
        let tree = TreeBuilder::new()
            .with_config(TreeBuilderConfig::default())
            .with_tracks(self.tracks.clone())
            .with_metadata(metadata)
            .with_plans(plans)
            .with_sessions(runtime_sessions)
            .with_external_sessions(external_sessions)
            .build()?;

        // Preserve expansion state from old tree
        let old_expanded = std::mem::take(&mut self.state.normalized_tree.expanded_nodes);
        let old_selected = self.state.normalized_tree.selected_node.clone();

        // Update tree
        self.state.normalized_tree = tree;

        // Restore expansion state for nodes that still exist
        for node_id in old_expanded {
            if self.state.normalized_tree.nodes.contains_key(&node_id) {
                self.state.normalized_tree.expanded_nodes.insert(node_id);
            }
        }

        if self.state.normalized_tree.expanded_nodes.is_empty() {
            for root_id in self.state.normalized_tree.root_ids.clone() {
                self.state.normalized_tree.expanded_nodes.insert(root_id);
            }
        }

        // Restore selection if it still exists
        if let Some(selected_id) = old_selected {
            if self.state.normalized_tree.nodes.contains_key(&selected_id) {
                self.state.normalized_tree.selected_node = Some(selected_id);
            }
        }

        if self.state.normalized_tree.selected_node.is_none() {
            self.sync_selection_from_legacy();
            if self.state.normalized_tree.selected_node.is_none() {
                self.state.normalized_tree.selected_node = self
                    .state
                    .normalized_tree
                    .visible_nodes()
                    .first()
                    .map(|(node_id, _)| node_id.clone());
            }
        }

        Ok(())
    }

    fn collect_runtime_sessions(&self) -> Vec<SessionState> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let orchestrate_base = PathBuf::from(home).join(".maestro").join("orchestrate");
        let mut sessions = Vec::new();

        if let Ok(entries) = std::fs::read_dir(orchestrate_base) {
            for entry in entries.flatten() {
                let session_path = entry.path().join("session.json");
                if !session_path.exists() {
                    continue;
                }

                if let Ok(content) = std::fs::read_to_string(&session_path) {
                    if let Ok(session) = serde_json::from_str::<SessionState>(&content) {
                        sessions.push(session);
                    }
                }
            }
        }

        sessions
    }

    /// Collect external sessions for tree building
    fn collect_external_sessions_for_tree(
        &self,
        external_sessions: &mut Vec<super::tree_builder::ExternalSession>,
    ) {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let orchestrate_base = PathBuf::from(home).join(".maestro").join("orchestrate");

        if let Ok(entries) = std::fs::read_dir(orchestrate_base) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let track_id = entry.file_name().to_string_lossy().to_string();

                        // Skip if already in tracks
                        if self.tracks.iter().any(|t| t.id == track_id) {
                            continue;
                        }

                        let session_path = entry.path().join("session.json");
                        if session_path.exists() {
                            if let Ok(content) = std::fs::read_to_string(&session_path) {
                                if let Ok(session) = serde_json::from_str::<SessionState>(&content)
                                {
                                    // Skip completed/failed sessions
                                    if matches!(
                                        session.status,
                                        SessionStatus::Completed
                                            | SessionStatus::Failed
                                            | SessionStatus::Interrupted
                                            | SessionStatus::Idle
                                    ) {
                                        continue;
                                    }

                                    external_sessions.push(super::tree_builder::ExternalSession {
                                        id: session.session_id.clone(),
                                        track_id: track_id.clone(),
                                        task_id: String::new(),
                                        title: format!("Session {}", session.session_id),
                                        status: super::normalized_model::ConductorNodeStatus::from(
                                            session.status,
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Toggle expansion using TreeNodeId
    pub fn toggle_node_expansion(&mut self, node_id: &TreeNodeId) {
        self.state.normalized_tree.toggle_expanded(node_id);
    }

    /// Check if a node is expanded using TreeNodeId
    pub fn is_node_expanded(&self, node_id: &TreeNodeId) -> bool {
        self.state.normalized_tree.is_expanded(node_id)
    }

    /// Get the currently selected node from the normalized tree
    pub fn get_selected_node(&self) -> Option<&Arc<dyn super::ConductorNode>> {
        self.state
            .normalized_tree
            .selected_node
            .as_ref()
            .and_then(|id| self.state.normalized_tree.nodes.get(id))
    }

    /// Set the selected node in the normalized tree
    pub fn set_selected_node(&mut self, node_id: Option<TreeNodeId>) {
        self.state.normalized_tree.set_selected(node_id);
    }

    /// Get all visible nodes from the normalized tree
    pub fn get_visible_nodes(&self) -> Vec<Arc<dyn super::ConductorNode>> {
        self.state
            .normalized_tree
            .visible_nodes()
            .into_iter()
            .map(|(_, node)| node)
            .collect()
    }

    fn sync_selection_from_legacy(&mut self) {
        let Some(current_id) = self.get_current_selection_id() else {
            self.state.normalized_tree.selected_node = None;
            return;
        };

        let candidate_ids = [
            TreeNodeId::track(&current_id),
            TreeNodeId::task(&current_id),
            TreeNodeId::session(&current_id),
        ];

        for node_id in candidate_ids {
            if self.state.normalized_tree.nodes.contains_key(&node_id) {
                self.state.normalized_tree.selected_node = Some(node_id);
                return;
            }
        }
    }
}

/// Render the Conductor pane
pub fn render_conductor(
    frame: &mut Frame,
    area: Rect,
    pane: &mut ConductorPane,
    theme: &crate::theme::Theme,
) {
    // 3-tier vertical layout: Header, Main, Footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(0),    // Main Content
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Synchronize basic state for header rendering
    if let Some(track_idx) = pane.get_selected_track_index() {
        if let Ok(Some(plan)) = pane.load_track_plan_internal(track_idx) {
            let (total, completed) = count_tasks_recursive(&plan.tasks);
            pane.state.total_tasks = total;
            pane.state.tasks_completed = completed;
        }

        if matches!(
            pane.state.status,
            super::model::ConductorStatus::Ready | super::model::ConductorStatus::Idle
        ) {
            pane.state.current_track = Some(pane.tracks[track_idx].id.clone());
        }
    }

    pane.state.current_iteration = pane.current_iteration;

    // Render Header
    super::header::render_header(frame, chunks[0], &pane.state);

    // Split main content into left (track/task tree) and right (details/output)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    // Split left panel vertically: Track/Task tree (top) and Subagent Tree (bottom)
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60), // Track/Task tree
            Constraint::Percentage(40), // Subagent Tree
        ])
        .split(main_chunks[0]);

    // Left panel top: Track/Task tree
    crate::conductor::track_tree::render_track_tree_normalized(frame, left_chunks[0], pane, theme);

    // Left panel bottom: Logs (Output mode essentially, but pinned to bottom-left)
    render_logs_pane(frame, left_chunks[1], pane, theme);

    // Right panel: Details/Output/Prompt based on mode
    let right_chunks = if pane.details_mode == crate::conductor::model::DetailsViewMode::Details {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70), // Details
                Constraint::Percentage(30), // Iteration History
            ])
            .split(main_chunks[1])
    } else {
        vec![main_chunks[1]].into()
    };

    if pane.details_mode == crate::conductor::model::DetailsViewMode::Details {
        crate::conductor::details_panel::render_details_panel(frame, right_chunks[0], pane, theme);
        crate::conductor::iteration_history::render_iteration_history(
            frame,
            right_chunks[1],
            pane,
            theme,
        );
    } else {
        crate::conductor::details_panel::render_details_panel(frame, right_chunks[0], pane, theme);
    }

    // Render Subagent Tree as a floating or secondary pane if there are active subagents
    if !pane.state.subagents.is_empty()
        && pane.details_mode == crate::conductor::model::DetailsViewMode::Details
    {
        // Overlay it or put it in another chunk. For now, we've replaced it with logs in the bottom-left.
        // Let's stick with the spec: bottom-left is for logs.
    }

    if pane.show_iteration_popup {
        crate::conductor::iteration_history::render_iteration_popup(frame, area, pane, theme);
    }

    // Render Setup Wizard if open (dominant overlay)
    if pane.setup.show_setup_wizard {
        render_setup_wizard(frame, area, pane, theme);
        return; // Setup wizard is dominant, don't render other UI
    }

    // Render Dashboard overlay if open
    if pane.show_dashboard {
        crate::conductor::dashboard::render_dashboard(frame, area, &pane.state);
    }

    // Render Project Selector if open
    if pane.state.show_project_selector {
        crate::conductor::project_selector::render_project_selector(frame, area, &pane.state);
    }

    // Render Footer
    super::footer::render_footer(frame, chunks[2], &pane.state);
}

fn count_tasks_recursive(tasks: &[Task]) -> (usize, usize) {
    let mut total = 0;
    let mut completed = 0;
    for task in tasks {
        total += 1;
        if task.status == TrackStatus::Completed {
            completed += 1;
        }
        let (sub_total, sub_completed) = count_tasks_recursive(&task.subtasks);
        total += sub_total;
        completed += sub_completed;
    }
    (total, completed)
}

fn render_logs_pane(
    frame: &mut Frame,
    area: Rect,
    pane: &mut ConductorPane,
    theme: &crate::theme::Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Runtime Logs ")
        .border_style(if pane.output_focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.muted)
        });

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let logs_text: Vec<Line> = pane
        .state
        .runtime_logs
        .iter()
        .rev()
        .take(area.height as usize)
        .into_iter()
        .rev()
        .map(|entry| {
            let style = match entry.level {
                crate::conductor::model::RuntimeLogLevel::Info => Style::default().fg(theme.muted),
                crate::conductor::model::RuntimeLogLevel::Success => {
                    Style::default().fg(Color::Green)
                }
                crate::conductor::model::RuntimeLogLevel::Warning => {
                    Style::default().fg(Color::Yellow)
                }
                crate::conductor::model::RuntimeLogLevel::Error => Style::default().fg(Color::Red),
            };

            let prefix = entry
                .iteration
                .map(|iter| format!("#{} ", iter))
                .unwrap_or_default();

            Line::from(vec![
                Span::styled(
                    format!("{} ", entry.timestamp.format("%H:%M:%S")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(prefix, Style::default().fg(theme.accent)),
                Span::styled(&entry.summary, style),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(logs_text).wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(paragraph, inner_area);
}

/// Render the setup wizard overlay
fn render_setup_wizard(
    frame: &mut Frame,
    area: Rect,
    pane: &mut ConductorPane,
    theme: &crate::theme::Theme,
) {
    // Clear the entire area, then paint a dark scrim background
    frame.render_widget(Clear, area);
    let scrim = Block::default().style(Style::default().bg(Color::Rgb(15, 15, 20)));
    frame.render_widget(scrim, area);

    // Center the wizard content
    let wizard_width = 60.min(area.width.saturating_sub(4));
    let wizard_height = 20.min(area.height.saturating_sub(4));

    let x = (area.width.saturating_sub(wizard_width)) / 2;
    let y = (area.height.saturating_sub(wizard_height)) / 2;

    let wizard_area = Rect {
        x: area.x + x,
        y: area.y + y,
        width: wizard_width,
        height: wizard_height,
    };

    let status = pane.setup.status.as_ref();

    match pane.setup.wizard_step {
        0 => render_wizard_welcome(frame, wizard_area, status, theme),
        1 => render_wizard_tool_selection(frame, wizard_area, pane, theme),
        2 => render_wizard_confirm(frame, wizard_area, pane, theme),
        _ => render_wizard_welcome(frame, wizard_area, status, theme),
    }
}

/// Render welcome step explaining why setup is needed
fn render_wizard_welcome(
    frame: &mut Frame,
    area: Rect,
    status: Option<&SetupStatus>,
    theme: &crate::theme::Theme,
) {
    let block = Block::default()
        .title(" Conductor Setup ")
        .title_style(Style::default().fg(theme.accent).bold())
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![
        Line::from(Span::styled(
            "Welcome to Conductor!",
            Style::default().fg(theme.accent).bold(),
        )),
        Line::from(""),
        Line::from("Before launching tracks, we need to ensure"),
        Line::from("your environment is properly configured."),
        Line::from(""),
    ];

    if let Some(status) = status {
        if !status.tracks_md_exists {
            lines.push(Line::from(Span::styled(
                "  • tracks.md not found",
                Style::default().fg(Color::Red),
            )));
            lines.push(Line::from(Span::styled(
                "    Run: maestro newTrack",
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(""));
        }

        if status.available_tools.is_empty() {
            lines.push(Line::from(Span::styled(
                "  • No AI tools detected",
                Style::default().fg(Color::Red),
            )));
            lines.push(Line::from(Span::styled(
                "    Install: iflow, claude, gemini, or qwen",
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(""));
        }

        let missing = status.missing_requirements();
        if !missing.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Missing Requirements:",
                Style::default().fg(Color::Red).bold(),
            )));
            for req in missing {
                lines.push(Line::from(Span::styled(
                    format!("  • {}", req),
                    Style::default().fg(Color::Yellow),
                )));
            }
            lines.push(Line::from(""));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press Enter to continue, Esc to dismiss",
        Style::default().fg(theme.muted),
    )));

    let paragraph = Paragraph::new(lines)
        .wrap(ratatui::widgets::Wrap { trim: true })
        .alignment(ratatui::layout::Alignment::Left);

    frame.render_widget(paragraph, inner);
}

/// Render tool selection step
fn render_wizard_tool_selection(
    frame: &mut Frame,
    area: Rect,
    pane: &mut ConductorPane,
    theme: &crate::theme::Theme,
) {
    let block = Block::default()
        .title(" Select AI Tool ")
        .title_style(Style::default().fg(theme.accent).bold())
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let status = pane.setup.status.as_ref();
    let empty_tools = vec![];
    let available_tools = status.map(|s| &s.available_tools).unwrap_or(&empty_tools);

    let mut lines = vec![
        Line::from(Span::styled(
            "Select your primary AI tool:",
            Style::default().fg(theme.accent).bold(),
        )),
        Line::from(""),
    ];

    let tools: Vec<(AgentTool, bool)> = vec![
        (
            AgentTool::Iflow,
            available_tools.contains(&AgentTool::Iflow),
        ),
        (
            AgentTool::Claude,
            available_tools.contains(&AgentTool::Claude),
        ),
        (
            AgentTool::Gemini,
            available_tools.contains(&AgentTool::Gemini),
        ),
        (AgentTool::Qwen, available_tools.contains(&AgentTool::Qwen)),
        (
            AgentTool::OpenCode,
            available_tools.contains(&AgentTool::OpenCode),
        ),
    ];

    for (i, (tool, available)) in tools.iter().enumerate() {
        let selected = pane.setup.selected_tool == Some(*tool);
        let status_mark = if selected {
            "●"
        } else if *available {
            "○"
        } else {
            "✗"
        };

        let style = if selected {
            Style::default().fg(theme.accent).bold()
        } else if *available {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", i + 1), style),
            Span::styled(status_mark, style),
            Span::styled(
                format!(
                    " {} - {}",
                    tool.as_str(),
                    if *available {
                        "available"
                    } else {
                        "not installed"
                    }
                ),
                style,
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press 1-5 to select, Enter to confirm, Esc to cancel",
        Style::default().fg(theme.muted),
    )));

    let paragraph = Paragraph::new(lines)
        .wrap(ratatui::widgets::Wrap { trim: false })
        .alignment(ratatui::layout::Alignment::Left);

    frame.render_widget(paragraph, inner);
}

/// Render confirmation step
fn render_wizard_confirm(
    frame: &mut Frame,
    area: Rect,
    pane: &mut ConductorPane,
    theme: &crate::theme::Theme,
) {
    let block = Block::default()
        .title(" Setup Complete ")
        .title_style(Style::default().fg(theme.accent).bold())
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let tool_name = pane
        .setup
        .selected_tool
        .map(|t| t.as_str().to_string())
        .unwrap_or_else(|| "none".to_string());

    let lines = vec![
        Line::from(Span::styled(
            "Setup Summary:",
            Style::default().fg(theme.accent).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Selected Tool: ", Style::default().fg(Color::White)),
            Span::styled(&tool_name, Style::default().fg(theme.accent).bold()),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from("You can now launch tracks with Conductor."),
        Line::from(""),
        Line::from("Key bindings when running:"),
        Line::from("  s - Start track"),
        Line::from("  p - Pause"),
        Line::from("  r - Resume"),
        Line::from("  ? - Status"),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to dismiss, Esc to close",
            Style::default().fg(theme.muted),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .wrap(ratatui::widgets::Wrap { trim: true })
        .alignment(ratatui::layout::Alignment::Left);

    frame.render_widget(paragraph, inner);
}
