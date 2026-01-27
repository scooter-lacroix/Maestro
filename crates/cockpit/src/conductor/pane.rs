//! Conductor Pane - Ralph-style autonomous task execution
//!
//! This module provides the Conductor tab for the Cockpit TUI.
//! It integrates with the Maestro orchestrate engine to display
//! tracks, tasks, and execution progress in a command-center style UI.
//!
//! Inspired by Ralph TUI (https://ghuntley.com/ralph/)

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use std::path::PathBuf;

use leindex_core::orchestrate::{
    model::{Track, TrackPlan, Task, TrackStatus, SessionStatus, LoopMode},
    parser::{parse_tracks_md, parse_plan_md},
    setup::{SetupStatus, detect_setup_status, AgentTool},
};

use super::model::ConductorState;
use crate::maestro_paths::MaestroProject;

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
    /// Setup state
    pub setup: SetupState,
    /// Whether to show the dashboard overlay
    pub show_dashboard: bool,
    /// Current Maestro project info
    pub current_project: Option<MaestroProject>,
}

impl Default for ConductorPane {
    fn default() -> Self {
        Self {
            state: ConductorState::default(),
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
            setup: Default::default(),
            show_dashboard: false,
            current_project: None,
        }
    }
}

impl ConductorPane {
    /// Create a new conductor pane
    pub fn new(tracks_dir: PathBuf) -> Self {
        let mut pane = Self {
            tracks_dir,
            ..Default::default()
        };
        pane.load_tracks().ok();
        pane
    }

    /// Create a conductor pane with auto-resolved tracks directory.
    /// Uses maestro_paths to discover the tracks directory from current working dir
    /// and active tmux panes.
    pub fn auto_discover() -> Self {
        let projects = crate::maestro_paths::discover_all_projects();
        
        // Prefer the project matching current working directory, or first available
        let project = if let Ok(cwd) = std::env::current_dir() {
            projects.iter()
                .find(|p| cwd.starts_with(&p.root_dir))
                .or_else(|| projects.first())
                .cloned()
        } else {
            projects.first().cloned()
        };

        let tracks_dir = project.as_ref().map(|p| p.tracks_dir.clone()).unwrap_or_else(|| PathBuf::from("."));
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
        self.discover_external_sessions(&mut loaded_tracks);
        
        self.tracks = loaded_tracks;
        Ok(())
    }

    fn discover_external_sessions(&self, tracks: &mut Vec<Track>) {
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
                                // Create a placeholder Track for this external session
                                tracks.push(Track {
                                    id: track_id.clone(),
                                    description: "CLI/External Session".to_string(),
                                    status: leindex_core::orchestrate::model::TrackStatus::InProgress,
                                    link_path: entry.path(), // Use orchestrate dir as link path for now
                                    metadata: None,
                                    plan: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Reload tracks (call when tracks_dir changes or to refresh)
    pub fn reload(&mut self) {
        self.invalidate_plan_cache();
        self.tracks.clear();
        self.selected_index = 0;
        let _ = self.load_tracks();
    }
    
    /// Set the tracks directory and reload
    pub fn set_tracks_dir(&mut self, tracks_dir: PathBuf) {
        self.tracks_dir = tracks_dir;
        self.reload();
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
    }

    /// Get the flat list of selectable items for the tree
    pub fn get_selectable_items(&mut self) -> Vec<crate::conductor::model::SelectableItem> {
        let mut items = Vec::new();
        
        // We need to work with a clone of tracks to avoid borrowing issues while we might need to load plans
        let tracks_clone = self.tracks.clone();
        
        for (idx, track) in tracks_clone.iter().enumerate() {
            let is_master = track.id.contains("master") || track.is_master();
            let is_external = track.description == "CLI/External Session";

            items.push(crate::conductor::model::SelectableItem::Track {
                index: idx,
                id: track.id.clone(),
                is_master,
                is_external,
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
        items: &mut Vec<crate::conductor::model::SelectableItem>
    ) {
        let is_expanded = self.expanded_tasks.contains(&task.id);
        let has_children = !task.subtasks.is_empty();
        
        items.push(crate::conductor::model::SelectableItem::Task {
            id: task.id.clone(),
            title: task.title.clone(),
            depth,
            status: task.status,
            has_children,
            is_expanded,
        });
        
        if is_expanded {
            for subtask in &task.subtasks {
                self.add_tasks_to_selectable_items(subtask, depth + 1, items);
            }
        }
    }

    /// Internal helper to load plan without borrowing self.tracks
    fn load_track_plan_internal(&mut self, track_idx: usize) -> Result<Option<TrackPlan>, Box<dyn std::error::Error>> {
        if track_idx >= self.tracks.len() {
            return Ok(None);
        }

        if self.cached_plan_track_index == Some(track_idx) {
            if let Some(ref plan) = self.cached_plan {
                return Ok(Some(plan.clone()));
            }
        }

        let track = &self.tracks[track_idx];
        let plan_path = track.link_path.join("plan.md");

        if !plan_path.exists() {
            return Ok(None);
        }

        let plan = parse_plan_md(&plan_path)?;
        self.cached_plan = Some(plan.clone());
        self.cached_plan_track_index = Some(track_idx);
        Ok(Some(plan))
    }

    /// Toggle expansion of the selected task or track
    pub fn toggle_task_expansion(&mut self, id: &str) {
        if self.expanded_tasks.contains(id) {
            self.expanded_tasks.remove(id);
        } else {
            self.expanded_tasks.insert(id.to_string());
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
    }

    /// Get currently selected track index
    pub fn get_selected_track_index(&mut self) -> Option<usize> {
        let items = self.get_selectable_items();
        if items.is_empty() { return None; }
        
        let idx = self.selected_index.min(items.len() - 1);
        for i in (0..=idx).rev() {
            if let crate::conductor::model::SelectableItem::Track { index, .. } = items[i] {
                return Some(index);
            }
        }
        None
    }

    /// Select next track
    pub fn next_track(&mut self) {
        if self.tracks.is_empty() { return; }
        
        let current_track_idx = self.get_selected_track_index().unwrap_or(0);
        let next_idx = (current_track_idx + 1) % self.tracks.len();
        
        // Find the index of the next track in the flattened list
        let items = self.get_selectable_items();
        for (i, item) in items.iter().enumerate() {
            if let crate::conductor::model::SelectableItem::Track { index, .. } = item {
                if *index == next_idx {
                    self.selected_index = i;
                    break;
                }
            }
        }
    }

    /// Select previous track
    pub fn prev_track(&mut self) {
        if self.tracks.is_empty() { return; }
        
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
            self.iteration_output = self.iteration_output
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
    pub fn get_start_command(&mut self, tool: Option<&str>, dangerous: bool, sandbox: bool) -> String {
        let track_idx = match self.get_selected_track_index() {
            Some(idx) => idx,
            None => return "// No track selected".to_string(),
        };

        let track_id = &self.tracks[track_idx].id;
        let tool = tool.unwrap_or("claude");
        let mode_str = match self.loop_mode {
            LoopMode::Planning => "planning",
            LoopMode::Building => "building",
        };

        let mut cmd = format!("maestro orchestrate start {} --mode {} --tool {}", track_id, mode_str, tool);

        if dangerous {
            cmd.push_str(" --dangerous");
        }

        if sandbox {
            cmd.push_str(" --sandbox");
        }

        cmd
    }

    /// Get the recommended pause command for the current track
    pub fn get_pause_command(&mut self) -> String {
        let track_idx = match self.get_selected_track_index() {
            Some(idx) => idx,
            None => return "// No track selected".to_string(),
        };

        let track_id = &self.tracks[track_idx].id;
        format!("maestro orchestrate pause {}", track_id)
    }

    /// Get the recommended resume command for the current track
    pub fn get_resume_command(&mut self) -> String {
        let track_idx = match self.get_selected_track_index() {
            Some(idx) => idx,
            None => return "// No track selected".to_string(),
        };

        let track_id = &self.tracks[track_idx].id;
        format!("maestro orchestrate resume {}", track_id)
    }

    /// Get the recommended status command for the current track
    pub fn get_status_command(&mut self) -> String {
        let track_idx = match self.get_selected_track_index() {
            Some(idx) => idx,
            None => return "maestro orchestrate status".to_string(),
        };

        let track_id = &self.tracks[track_idx].id;
        format!("maestro orchestrate status {}", track_id)
    }

    /// Get the command to create a new track
    pub fn get_new_track_command(&self) -> String {
        "maestro newTrack".to_string()
    }
}

/// Render the Conductor pane
pub fn render_conductor(frame: &mut Frame, area: Rect, pane: &mut ConductorPane, theme: &crate::theme::Theme) {
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
    let items = pane.get_selectable_items();
    if !items.is_empty() {
        let selected_idx = pane.selected_index.min(items.len() - 1);
        
        // Update total/completed tasks based on selected track
        if let Some(track_idx) = pane.get_selected_track_index() {
             if let Ok(Some(plan)) = pane.load_track_plan_internal(track_idx) {
                  let (total, completed) = count_tasks_recursive(&plan.tasks);
                  pane.state.total_tasks = total;
                  pane.state.tasks_completed = completed;
             }
        }

        // If not running, we can show selection in header.
        // If running, polling.rs will handle current_task.
        if matches!(pane.state.status, super::model::ConductorStatus::Ready | super::model::ConductorStatus::Idle) {
            match &items[selected_idx] {
                crate::conductor::model::SelectableItem::Track { id, .. } => {
                    pane.state.current_track = Some(id.clone());
                    pane.state.current_task = None;
                }
                crate::conductor::model::SelectableItem::Task { id, .. } => {
                    pane.state.current_task = Some(id.clone());
                    if let Some(track_idx) = pane.get_selected_track_index() {
                        pane.state.current_track = Some(pane.tracks[track_idx].id.clone());
                    }
                }
            }
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
    crate::conductor::track_tree::render_track_tree(frame, left_chunks[0], pane, theme);

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
        crate::conductor::iteration_history::render_iteration_history(frame, right_chunks[1], pane, theme);
    } else {
        crate::conductor::details_panel::render_details_panel(frame, right_chunks[0], pane, theme);
    }

    // Render Subagent Tree as a floating or secondary pane if there are active subagents
    if !pane.state.subagents.is_empty() && pane.details_mode == crate::conductor::model::DetailsViewMode::Details {
        // Overlay it or put it in another chunk. For now, we've replaced it with logs in the bottom-left.
        // Let's stick with the spec: bottom-left is for logs.
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
    super::footer::render_footer(frame, chunks[2]);
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

fn render_logs_pane(frame: &mut Frame, area: Rect, pane: &mut ConductorPane, theme: &crate::theme::Theme) {
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

    // Filter out common metadata lines to keep logs clean
    let filtered_logs: Vec<&String> = pane.iteration_output
        .iter()
        .rev()
        .filter(|line| !line.starts_with("--- Iteration"))
        .take(area.height as usize)
        .collect();

    let logs_text: Vec<Line> = filtered_logs
        .into_iter()
        .rev()
        .map(|line| {
            let line_upper = line.to_uppercase();
            let style = if line_upper.contains("ERROR") || line_upper.contains("FAIL") {
                Style::default().fg(Color::Red)
            } else if line.starts_with("---") {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.muted)
            };
            Line::from(Span::styled(line.as_str(), style))
        })
        .collect();

    let paragraph = Paragraph::new(logs_text).wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(paragraph, inner_area);
}
