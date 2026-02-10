//! Orchestrate Pane - Ralph-style autonomous task execution
//!
//! This module provides the Orchestrate tab for the Cockpit TUI.
//! It integrates with the Maestro orchestrate engine to display
//! tracks, tasks, and execution progress.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use std::path::PathBuf;

use leindex_core::orchestrate::{
    model::{LoopMode, SessionStatus, Task, Track, TrackPlan, TrackStatus},
    parser::{parse_plan_md, parse_tracks_md},
    setup::{detect_setup_status, AgentTool, SetupStatus},
};

/// Setup state for the orchestrate pane
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

/// State for the Orchestrate pane
pub struct OrchestratePane {
    /// List of tracks loaded from tracks.md
    pub tracks: Vec<Track>,
    /// Currently selected track index
    pub selected_track: usize,
    /// Selected task in the current track
    pub selected_task: Option<String>,
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
}

impl Default for OrchestratePane {
    fn default() -> Self {
        Self {
            tracks: Vec::new(),
            selected_track: 0,
            selected_task: None,
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
        }
    }
}

impl OrchestratePane {
    /// Create a new orchestrate pane
    pub fn new(tracks_dir: PathBuf) -> Self {
        let mut pane = Self {
            tracks_dir,
            ..Default::default()
        };
        pane.load_tracks().ok();
        pane
    }

    /// Load tracks from tracks.md
    pub fn load_tracks(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let tracks_path = self.tracks_dir.join("tracks.md");
        self.tracks = parse_tracks_md(&tracks_path)?;
        Ok(())
    }

    /// Load plan for the selected track (with caching)
    pub fn load_selected_track_plan(
        &mut self,
    ) -> Result<Option<TrackPlan>, Box<dyn std::error::Error>> {
        if self.tracks.is_empty() {
            return Ok(None);
        }

        // Check if we have a cached plan for the current track
        if self.cached_plan_track_index == Some(self.selected_track) {
            if let Some(ref plan) = self.cached_plan {
                return Ok(Some(plan.clone()));
            }
        }

        // Cache miss - load from disk
        let track = &self.tracks[self.selected_track];
        let plan_path = track.link_path.join("plan.md");

        if !plan_path.exists() {
            self.error_message = Some(format!("No plan.md found for track: {}", track.id));
            self.cached_plan = None;
            self.cached_plan_track_index = None;
            return Ok(None);
        }

        let plan = parse_plan_md(&plan_path)?;
        self.cached_plan = Some(plan.clone());
        self.cached_plan_track_index = Some(self.selected_track);
        Ok(Some(plan))
    }

    /// Invalidate the cached plan (call when plan.md changes externally)
    pub fn invalidate_plan_cache(&mut self) {
        self.cached_plan = None;
        self.cached_plan_track_index = None;
    }

    /// Toggle expansion of the selected task
    pub fn toggle_task_expansion(&mut self, task_id: &str) {
        if self.expanded_tasks.contains(task_id) {
            self.expanded_tasks.remove(task_id);
        } else {
            self.expanded_tasks.insert(task_id.to_string());
        }
    }

    /// Check if a task is expanded
    pub fn is_task_expanded(&self, task_id: &str) -> bool {
        self.expanded_tasks.contains(task_id)
    }

    /// Select next track
    pub fn next_track(&mut self) {
        if !self.tracks.is_empty() {
            self.selected_track = (self.selected_track + 1) % self.tracks.len();
            self.selected_task = None;
            self.expanded_tasks.clear();
            self.invalidate_plan_cache();
        }
    }

    /// Select previous track
    pub fn prev_track(&mut self) {
        if !self.tracks.is_empty() {
            self.selected_track = if self.selected_track == 0 {
                self.tracks.len() - 1
            } else {
                self.selected_track - 1
            };
            self.selected_task = None;
            self.expanded_tasks.clear();
            self.invalidate_plan_cache();
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
    pub fn get_start_command(&self, tool: Option<&str>, dangerous: bool, sandbox: bool) -> String {
        if self.tracks.is_empty() {
            return "// No tracks available".to_string();
        }

        let track_id = &self.tracks[self.selected_track].id;
        let tool = tool.unwrap_or("claude");
        let mode_str = match self.loop_mode {
            LoopMode::Planning => "planning",
            LoopMode::Building => "building",
        };

        let mut cmd = format!(
            "maestro orchestrate start {} --mode {} --tool {}",
            track_id, mode_str, tool
        );

        if dangerous {
            cmd.push_str(" --dangerous");
        }

        if sandbox {
            cmd.push_str(" --sandbox");
        }

        cmd
    }

    /// Get the recommended pause command for the current track
    pub fn get_pause_command(&self) -> String {
        if self.tracks.is_empty() {
            return "// No tracks available".to_string();
        }

        let track_id = &self.tracks[self.selected_track].id;
        format!("maestro orchestrate pause {}", track_id)
    }

    /// Get the recommended resume command for the current track
    pub fn get_resume_command(&self) -> String {
        if self.tracks.is_empty() {
            return "// No tracks available".to_string();
        }

        let track_id = &self.tracks[self.selected_track].id;
        format!("maestro orchestrate resume {}", track_id)
    }

    /// Get the recommended status command for the current track
    pub fn get_status_command(&self) -> String {
        if self.tracks.is_empty() {
            return "maestro orchestrate status".to_string();
        }

        let track_id = &self.tracks[self.selected_track].id;
        format!("maestro orchestrate status {}", track_id)
    }

    /// Get the command to create a new track
    pub fn get_new_track_command(&self) -> String {
        "maestro newTrack".to_string()
    }
}

/// Render the Orchestrate pane
pub fn render_orchestrate(
    frame: &mut Frame,
    area: Rect,
    pane: &mut OrchestratePane,
    theme: &crate::theme::Theme,
) {
    // Split into left (track/task tree) and right (details/output)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Left panel: Track/Task tree
    render_track_tree(frame, chunks[0], pane, theme);

    // Right panel: Task details and output
    render_task_details(frame, chunks[1], pane, theme);
}

/// Render the track/task tree (left panel)
fn render_track_tree(
    frame: &mut Frame,
    area: Rect,
    pane: &mut OrchestratePane,
    theme: &crate::theme::Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Tracks & Tasks ")
        .border_style(if !pane.output_focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.muted)
        });

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if pane.tracks.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from("  No tracks found."),
            Line::from(""),
            Line::from("  Ensure tracks.md exists"),
            Line::from("  in the project directory."),
        ];
        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.muted));
        frame.render_widget(paragraph, inner_area);
        return;
    }

    // Build tree items
    let mut items: Vec<ListItem<'static>> = Vec::new();

    // Collect track data first to avoid borrow issues
    let track_data: Vec<(usize, TrackStatus, String)> = pane
        .tracks
        .iter()
        .enumerate()
        .map(|(idx, track)| (idx, track.status, track.id.clone()))
        .collect();

    for (idx, status, track_id) in track_data {
        let is_selected = idx == pane.selected_track;

        // Track header
        let status_symbol = match status {
            TrackStatus::Pending => "[ ]",
            TrackStatus::InProgress => "[~]",
            TrackStatus::Completed => "[x]",
        };

        let line = format!("{} {}", status_symbol, track_id);
        let style = if is_selected {
            Style::default().fg(theme.accent).bold()
        } else {
            Style::default().fg(theme.fg)
        };
        items.push(ListItem::new(Span::styled(line, style)));

        // If selected and has plan, show tasks
        if is_selected {
            // Load the plan outside of the closure to avoid borrow issues
            let _ = pane.load_selected_track_plan().ok().map(|plan| {
                if let Some(plan) = plan {
                    for task in &plan.tasks {
                        render_task_tree_recursive(task, 1, pane, &mut items, theme);
                    }
                }
            });
        }
    }

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg),
        )
        .highlight_symbol(">> ");

    frame.render_widget(list, inner_area);
}

/// Recursively render task tree
fn render_task_tree_recursive(
    task: &Task,
    depth: usize,
    pane: &OrchestratePane,
    items: &mut Vec<ListItem<'static>>,
    theme: &crate::theme::Theme,
) {
    let indent = "  ".repeat(depth);
    let status_symbol = match task.status {
        TrackStatus::Pending => "[ ]".to_string(),
        TrackStatus::InProgress => "[~]".to_string(),
        TrackStatus::Completed => "[x]".to_string(),
    };

    let is_expanded = pane.is_task_expanded(&task.id);
    let has_children = !task.subtasks.is_empty();
    let expand_symbol = if has_children {
        if is_expanded {
            "[-]"
        } else {
            "[+]"
        }
    } else {
        if task.status == TrackStatus::Completed {
            "[✓]"
        } else {
            "   "
        }
    };

    // Build owned strings to avoid lifetime issues
    let line_text = format!(
        "{}{}{} {}{}",
        indent, expand_symbol, status_symbol, " ", task.title
    );
    items.push(ListItem::new(Span::styled(
        line_text,
        Style::default().fg(theme.fg),
    )));

    // Show subtasks if expanded
    if is_expanded {
        for subtask in &task.subtasks {
            render_task_tree_recursive(subtask, depth + 1, pane, items, theme);
        }
    }
}

/// Render task details and output (right panel)
fn render_task_details(
    frame: &mut Frame,
    area: Rect,
    pane: &mut OrchestratePane,
    theme: &crate::theme::Theme,
) {
    // Split into details (top) and output (bottom)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(20), Constraint::Min(0)])
        .split(area);

    // Task details
    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Task Details & Commands ")
        .border_style(if !pane.output_focused {
            Style::default().fg(theme.muted)
        } else {
            Style::default().fg(theme.accent)
        });

    let details_text = if pane.tracks.is_empty() {
        vec![
            Line::from(""),
            Line::from("  No track selected."),
            Line::from(""),
            Line::from("  Press 'n' to create a new track"),
            Line::from("  or run: maestro newTrack"),
            Line::from(""),
        ]
    } else {
        let track = &pane.tracks[pane.selected_track];
        let start_cmd = pane.get_start_command(Some("claude"), false, false);
        vec![
            Line::from(vec![
                Span::styled("Track: ", Style::default().fg(theme.accent_alt)),
                Span::styled(&track.id, Style::default().bold()),
            ]),
            Line::from(vec![
                Span::styled("Mode: ", Style::default().fg(theme.accent_alt)),
                Span::styled(format!("{:?}", pane.loop_mode), Style::default()),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(theme.accent_alt)),
                Span::styled(
                    format!("{:?}", pane.session_status),
                    Style::default().fg(match pane.session_status {
                        SessionStatus::Running => Color::Green,
                        SessionStatus::Paused => Color::Yellow,
                        SessionStatus::Completed => Color::Blue,
                        SessionStatus::Failed | SessionStatus::Interrupted => Color::Red,
                        _ => Color::Gray,
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled("Iteration: ", Style::default().fg(theme.accent_alt)),
                Span::styled(format!("{}", pane.current_iteration), Style::default()),
            ]),
            Line::from(""),
            Line::from("Commands:"),
            Line::from(vec![
                Span::styled("  [s] Start: ", Style::default().fg(theme.muted)),
                Span::styled(start_cmd, Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("  [p] Pause:  ", Style::default().fg(theme.muted)),
                Span::styled(pane.get_pause_command(), Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("  [r] Resume: ", Style::default().fg(theme.muted)),
                Span::styled(pane.get_resume_command(), Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("  [?] Status:  ", Style::default().fg(theme.muted)),
                Span::styled(pane.get_status_command(), Style::default().fg(theme.fg)),
            ]),
            Line::from(""),
        ]
    };

    let details = Paragraph::new(details_text).block(details_block);
    frame.render_widget(details, chunks[0]);

    // Output panel
    let output_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(if pane.output_focused {
            " Live Output (Focused) "
        } else {
            " Live Output "
        })
        .border_style(if pane.output_focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.muted)
        });

    let output_text: Vec<Line> = pane
        .iteration_output
        .iter()
        .map(|line| Line::from(line.as_str()))
        .collect();

    let output = Paragraph::new(output_text)
        .block(output_block)
        .wrap(Wrap { trim: false })
        .scroll((pane.output_scroll, 0));

    frame.render_widget(output, chunks[1]);
}
