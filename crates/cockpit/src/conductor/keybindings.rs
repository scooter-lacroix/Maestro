use crate::conductor::modals::Modal;
use crate::conductor::model::ConductorStatus;
use crate::conductor::pane::CommandArgs;
use crate::conductor::pane::ConductorPane;
use crate::toast::ToastLevel;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use leindex_core::orchestrate::model::LoopMode;

/// Result of a key event handling
pub enum ConductorAction {
    /// Event was not handled by Conductor
    None,
    /// Event was handled, no further action needed
    Handled,
    /// Event was handled, display a toast notification
    Toast { message: String, level: ToastLevel },
    /// Cycle to the next theme
    CycleTheme,
    /// Store a memory with content and category
    StoreMemory {
        content: String,
        category: leindex_core::memory::models::MemoryCategory,
    },
    /// Delete a memory by ID
    DeleteMemory { id: i64 },
}

/// Helper to create info toast action
impl ConductorAction {
    pub fn info(msg: impl Into<String>) -> Self {
        ConductorAction::Toast {
            message: msg.into(),
            level: ToastLevel::Info,
        }
    }

    pub fn success(msg: impl Into<String>) -> Self {
        ConductorAction::Toast {
            message: msg.into(),
            level: ToastLevel::Success,
        }
    }

    pub fn warning(msg: impl Into<String>) -> Self {
        ConductorAction::Toast {
            message: msg.into(),
            level: ToastLevel::Warning,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        ConductorAction::Toast {
            message: msg.into(),
            level: ToastLevel::Error,
        }
    }
}

/// Handle Conductor-specific key events
pub fn handle_key_event(pane: &mut ConductorPane, key: KeyEvent) -> ConductorAction {
    // 0. Handle modals first (they absorb all input when visible)
    if pane.steering_modal.visible {
        match pane.steering_modal.handle_key(key) {
            super::input_modal::InputAction::Submit(text) => {
                if !text.trim().is_empty() {
                    if let Some(track_id) = pane.state.current_track.clone() {
                        let _ = send_control_command(
                            &track_id,
                            ControlCommandType::Steer { message: text },
                        );
                        return ConductorAction::success("Steering message sent");
                    }
                }
                return ConductorAction::Handled;
            }
            super::input_modal::InputAction::Cancel => return ConductorAction::Handled,
            super::input_modal::InputAction::Handled => return ConductorAction::Handled,
            super::input_modal::InputAction::None => return ConductorAction::Handled,
        }
    }

    if pane.iter_modal.visible {
        match pane.iter_modal.handle_key(key) {
            super::input_modal::InputAction::Submit(text) => {
                if let Ok(max) = text.trim().parse::<u64>() {
                    if let Some(track_id) = pane.state.current_track.clone() {
                        let _ = send_control_command(
                            &track_id,
                            ControlCommandType::SetMaxIterations { max },
                        );
                        return ConductorAction::success(format!("Max iterations set to {}", max));
                    }
                }
                return ConductorAction::Handled;
            }
            super::input_modal::InputAction::Cancel => return ConductorAction::Handled,
            super::input_modal::InputAction::Handled => return ConductorAction::Handled,
            super::input_modal::InputAction::None => return ConductorAction::Handled,
        }
    }

    if pane.selector_modal.visible {
        match pane.selector_modal.handle_key(key) {
            super::selector_modal::SelectorAction::Selected(value) => {
                let title = pane.selector_modal.title.clone();
                if let Some(track_id) = pane.state.current_track.clone() {
                    if title.contains("Error Strategy") {
                        let _ = send_control_command(
                            &track_id,
                            ControlCommandType::SetErrorStrategy { strategy: value },
                        );
                        return ConductorAction::success("Error strategy updated");
                    } else if title.contains("Agent") {
                        let _ = send_control_command(
                            &track_id,
                            ControlCommandType::SwitchAgent {
                                tool: value,
                                _model: None,
                            },
                        );
                        return ConductorAction::success("Agent switched");
                    }
                }
                return ConductorAction::Handled;
            }
            super::selector_modal::SelectorAction::Cancel => return ConductorAction::Handled,
            super::selector_modal::SelectorAction::None => return ConductorAction::Handled,
        }
    }

    // 1. Handle Memory Browser if open
    if pane.memory_browser.is_visible()
        || pane.memory_browser.search_modal.is_visible()
        || pane.memory_browser.category_modal.is_visible()
        || pane.memory_browser.store_modal.is_visible()
        || pane.memory_browser.delete_modal.is_visible()
    {
        match pane.memory_browser.handle_key(key) {
            super::memory_browser::MemoryBrowserAction::Handled => return ConductorAction::Handled,
            super::memory_browser::MemoryBrowserAction::Close => {
                pane.memory_browser.hide();
                return ConductorAction::Handled;
            }
            super::memory_browser::MemoryBrowserAction::SearchFocused
            | super::memory_browser::MemoryBrowserAction::SearchSubmitted => {
                return ConductorAction::Handled;
            }
            super::memory_browser::MemoryBrowserAction::SearchCancelled => {
                return ConductorAction::Handled;
            }
            super::memory_browser::MemoryBrowserAction::CategoryOpened
            | super::memory_browser::MemoryBrowserAction::CategorySelected => {
                return ConductorAction::Handled;
            }
            super::memory_browser::MemoryBrowserAction::CategoryCancelled => {
                return ConductorAction::Handled;
            }
            super::memory_browser::MemoryBrowserAction::StoreOpened => {
                return ConductorAction::Handled;
            }
            super::memory_browser::MemoryBrowserAction::StoreMemory { content, category } => {
                // Return action to be handled by app.rs which has access to MemoryService
                return ConductorAction::StoreMemory { content, category };
            }
            super::memory_browser::MemoryBrowserAction::StoreCancelled => {
                return ConductorAction::Handled;
            }
            super::memory_browser::MemoryBrowserAction::DeleteOpened => {
                return ConductorAction::Handled;
            }
            super::memory_browser::MemoryBrowserAction::DeleteConfirmed { id } => {
                // Return action to be handled by app.rs which has access to MemoryService
                return ConductorAction::DeleteMemory { id };
            }
            super::memory_browser::MemoryBrowserAction::DeleteCancelled => {
                return ConductorAction::Handled;
            }
            super::memory_browser::MemoryBrowserAction::Ignored => return ConductorAction::None,
        }
    }

    // 2. Handle Project Selector if open

    // 1. Handle Project Selector if open
    if pane.state.show_project_selector {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) | (KeyModifiers::SHIFT, KeyCode::Char('P')) => {
                pane.state.show_project_selector = false;
                return ConductorAction::Handled;
            }
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                if !pane.state.available_projects.is_empty() {
                    pane.state.selected_project_index = (pane.state.selected_project_index + 1)
                        % pane.state.available_projects.len();
                }
                return ConductorAction::Handled;
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                if !pane.state.available_projects.is_empty() {
                    if pane.state.selected_project_index == 0 {
                        pane.state.selected_project_index = pane.state.available_projects.len() - 1;
                    } else {
                        pane.state.selected_project_index -= 1;
                    }
                }
                return ConductorAction::Handled;
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if !pane.state.available_projects.is_empty() {
                    let project =
                        pane.state.available_projects[pane.state.selected_project_index].clone();
                    pane.switch_project(project);
                }
                return ConductorAction::Handled;
            }
            // Allow global navigation keys to pass through even when modal is open
            (KeyModifiers::NONE, KeyCode::Tab)
            | (KeyModifiers::NONE, KeyCode::BackTab)
            | (KeyModifiers::SHIFT, KeyCode::BackTab) => return ConductorAction::None,
            (KeyModifiers::NONE, KeyCode::Char('1'..='8')) => return ConductorAction::None,
            (KeyModifiers::ALT, KeyCode::Char('1'..='8')) => return ConductorAction::None,
            _ => return ConductorAction::Handled, // Absorb other keys when modal is open
        }
    }

    // 2. Handle Dashboard if open
    if pane.show_dashboard {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) | (KeyModifiers::NONE, KeyCode::Char('d')) => {
                pane.show_dashboard = false;
                return ConductorAction::Handled;
            }
            // Allow global navigation keys to pass through
            (KeyModifiers::NONE, KeyCode::Tab)
            | (KeyModifiers::NONE, KeyCode::BackTab)
            | (KeyModifiers::SHIFT, KeyCode::BackTab) => return ConductorAction::None,
            (KeyModifiers::NONE, KeyCode::Char('1'..='8')) => return ConductorAction::None,
            (KeyModifiers::ALT, KeyCode::Char('1'..='8')) => return ConductorAction::None,
            _ => return ConductorAction::Handled,
        }
    }

    match (key.modifiers, key.code) {
        // Navigation
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            if pane.output_focused
                && pane.details_mode == crate::conductor::model::DetailsViewMode::Output
            {
                pane.scroll_output_up();
            } else if pane.output_focused
                && pane.details_mode == crate::conductor::model::DetailsViewMode::Prompt
            {
                // Prompt scrolling not yet implemented, treat same as output
                pane.scroll_output_up();
            } else {
                pane.move_selection(1);
            }
            ConductorAction::Handled
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            if pane.output_focused
                && pane.details_mode == crate::conductor::model::DetailsViewMode::Output
            {
                pane.scroll_output_down();
            } else if pane.output_focused
                && pane.details_mode == crate::conductor::model::DetailsViewMode::Prompt
            {
                // Prompt scrolling not yet implemented, treat same as output
                pane.scroll_output_down();
            } else {
                pane.move_selection(-1);
            }
            ConductorAction::Handled
        }
        (KeyModifiers::NONE, KeyCode::PageDown) => {
            if pane.output_focused
                && pane.details_mode == crate::conductor::model::DetailsViewMode::Output
            {
                for _ in 0..10 {
                    pane.scroll_output_up();
                }
            } else if pane.output_focused
                && pane.details_mode == crate::conductor::model::DetailsViewMode::Prompt
            {
                // Prompt scrolling not yet implemented
                for _ in 0..10 {
                    pane.scroll_output_up();
                }
            }
            ConductorAction::Handled
        }
        (KeyModifiers::NONE, KeyCode::PageUp) => {
            if pane.output_focused
                && pane.details_mode == crate::conductor::model::DetailsViewMode::Output
            {
                for _ in 0..10 {
                    pane.scroll_output_down();
                }
            } else if pane.output_focused
                && pane.details_mode == crate::conductor::model::DetailsViewMode::Prompt
            {
                // Prompt scrolling not yet implemented
                for _ in 0..10 {
                    pane.scroll_output_down();
                }
            }
            ConductorAction::Handled
        }

        // Track navigation (jump between tracks)
        (KeyModifiers::NONE, KeyCode::Char('o')) => {
            pane.next_track();
            ConductorAction::Handled
        }
        (KeyModifiers::SHIFT, KeyCode::Char('O')) | (KeyModifiers::CONTROL, KeyCode::Char('o')) => {
            pane.prev_track();
            ConductorAction::Handled
        }

        // Task interaction
        (_, KeyCode::Char(' ')) | (_, KeyCode::Enter) => {
            if !pane.output_focused {
                let items = pane.get_selectable_items();
                if let Some(item) = items.get(pane.selected_index) {
                    let id = match item {
                        crate::conductor::model::SelectableItem::Track { id, .. } => id.clone(),
                        crate::conductor::model::SelectableItem::Task { id, .. } => id.clone(),
                    };
                    pane.toggle_task_expansion(&id);
                    return ConductorAction::Handled;
                }
                ConductorAction::None
            } else {
                ConductorAction::None
            }
        }

        // Mode switching
        (KeyModifiers::ALT, KeyCode::Char('1')) => {
            pane.details_mode = crate::conductor::model::DetailsViewMode::Details;
            ConductorAction::Handled
        }
        (KeyModifiers::ALT, KeyCode::Char('2')) => {
            pane.details_mode = crate::conductor::model::DetailsViewMode::Output;
            ConductorAction::Handled
        }
        (KeyModifiers::ALT, KeyCode::Char('3')) => {
            pane.details_mode = crate::conductor::model::DetailsViewMode::Prompt;
            ConductorAction::Handled
        }
        (KeyModifiers::ALT, KeyCode::Char('4')) => {
            pane.details_mode = crate::conductor::model::DetailsViewMode::Parallel;
            ConductorAction::Handled
        }

        // Details mode switching (Legacy fallback removed to allow tab switching)

        // Output control (plain 'c' only, not Ctrl+C)
        (KeyModifiers::NONE, KeyCode::Char('c')) => {
            pane.clear_output();
            ConductorAction::Handled
        }

        // Dashboard toggle
        (_, KeyCode::Char('d')) => {
            pane.show_dashboard = !pane.show_dashboard;
            ConductorAction::Handled
        }

        // Dependency navigation: press 1-9 to jump to dependency
        (_, KeyCode::Char('1'))
        | (_, KeyCode::Char('2'))
        | (_, KeyCode::Char('3'))
        | (_, KeyCode::Char('4'))
        | (_, KeyCode::Char('5'))
        | (_, KeyCode::Char('6'))
        | (_, KeyCode::Char('7'))
        | (_, KeyCode::Char('8'))
        | (_, KeyCode::Char('9')) => {
            if !pane.output_focused {
                let items = pane.get_selectable_items();
                if let Some(item) = items.get(pane.selected_index) {
                    if let crate::conductor::model::SelectableItem::Task { dependencies, .. } = item
                    {
                        if let KeyCode::Char(c) = key.code {
                            if let Ok(idx) = c.to_string().parse::<usize>() {
                                if idx > 0 && idx <= dependencies.len() {
                                    let dep_id = &dependencies[idx - 1].task_id;
                                    // Find the dependency task in selectable items
                                    if let Some(dep_idx) = items.iter().position(|i| {
                                        if let crate::conductor::model::SelectableItem::Task {
                                            id,
                                            ..
                                        } = i
                                        {
                                            id == dep_id
                                        } else {
                                            false
                                        }
                                    }) {
                                        pane.selected_index = dep_idx;
                                        return ConductorAction::Handled;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ConductorAction::None
        }

        // Project Selector
        (KeyModifiers::SHIFT, KeyCode::Char('P')) => {
            pane.open_project_selector();
            ConductorAction::Handled
        }

        // Steering message modal (Ctrl+M)
        (KeyModifiers::CONTROL, KeyCode::Char('m')) => {
            pane.steering_modal.title = "Steering Message".to_string();
            pane.steering_modal.prompt_text = "Enter guidance for the next iteration:".to_string();
            pane.steering_modal.visible = true;
            ConductorAction::Handled
        }

        // Error strategy selector (e) - only when running
        (KeyModifiers::NONE, KeyCode::Char('e')) if pane.state.status != ConductorStatus::Ready => {
            pane.selector_modal = super::selector_modal::SelectorModal::new_with_descriptions(
                "Error Strategy",
                vec![
                    ("Retry", "retry", "Retry the task"),
                    ("Skip", "skip", "Skip this task"),
                    ("Abort", "abort", "Abort the track"),
                ],
            );
            pane.selector_modal.visible = true;
            ConductorAction::Handled
        }

        // Max iterations modal (i) - only when running
        (KeyModifiers::NONE, KeyCode::Char('i')) if pane.state.status != ConductorStatus::Ready => {
            pane.iter_modal.title = "Max Iterations".to_string();
            pane.iter_modal.prompt_text = "Enter max iterations (0 = unlimited):".to_string();
            pane.iter_modal.visible = true;
            ConductorAction::Handled
        }

        // Agent switch selector (a) - only when running
        (KeyModifiers::NONE, KeyCode::Char('a')) if pane.state.status != ConductorStatus::Ready => {
            pane.selector_modal = super::selector_modal::SelectorModal::new_with_descriptions(
                "Select Agent",
                vec![
                    ("Claude", "claude", "Anthropic's Claude"),
                    ("Gemini", "gemini", "Google's Gemini"),
                    ("Qwen", "qwen", "Alibaba's Qwen"),
                    ("OpenAI", "openai", "OpenAI's GPT"),
                ],
            );
            pane.selector_modal.visible = true;
            ConductorAction::Handled
        }

        // Cycle agent role (Shift+A) - always available
        (KeyModifiers::SHIFT, KeyCode::Char('A')) => {
            let role = pane.cycle_agent_role();
            let role_name = super::agent_executor::role_utils::role_display_name(&role);
            let role_desc = super::agent_executor::role_utils::role_description(&role);
            ConductorAction::info(format!("Agent role: {} - {}", role_name, role_desc))
        }

        // Cancel active execution (Ctrl+C)
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            // Note: This conflicts with Ctrl+C for abort. We handle it specially here.
            // Use async runtime to cancel
            let cancelled = pane
                .cancellation_token
                .as_ref()
                .map(|t| t.is_cancelled())
                .unwrap_or(false);
            if cancelled {
                ConductorAction::warning("Execution already cancelled")
            } else if let Some(ref token) = pane.cancellation_token {
                token.cancel();
                ConductorAction::success("Execution cancelled")
            } else {
                ConductorAction::warning("No active execution to cancel")
            }
        }

        // Mode toggle (m) - only when running
        (KeyModifiers::NONE, KeyCode::Char('m')) if pane.state.status != ConductorStatus::Ready => {
            let new_mode = if pane.state.loop_mode == LoopMode::Building {
                "planning"
            } else {
                "building"
            };
            if let Some(track_id) = pane.state.current_track.clone() {
                let _ = send_control_command(
                    &track_id,
                    ControlCommandType::SetLoopMode {
                        mode: new_mode.to_string(),
                    },
                );
                return ConductorAction::success(format!("Loop mode switched to {}", new_mode));
            }
            ConductorAction::warning("No track active")
        }

        // Focus toggle
        (KeyModifiers::ALT, KeyCode::Char('p')) => {
            pane.output_focused = !pane.output_focused;
            let msg = if pane.output_focused {
                "Output focused. Scroll with Arrows/PgUp/PgDn."
            } else {
                "Tracks focused."
            };
            ConductorAction::info(msg)
        }

        // Theme cycle (Shift+T)
        (KeyModifiers::SHIFT, KeyCode::Char('T')) => ConductorAction::CycleTheme,

        // Memory Browser (Shift+M) - always available
        (KeyModifiers::SHIFT, KeyCode::Char('M')) => {
            pane.memory_browser.show();
            ConductorAction::info("Memory browser opened. Press Esc to close.")
        }

        // Ralph behavior enforcement (CONTROL key combinations - check before single keys)
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => {
            // Retry current task
            if let Some(track_id) = pane.state.current_track.clone() {
                if let Some(current_task) = pane.state.current_task.clone() {
                    if let Err(e) = send_control_command(
                        &track_id,
                        ControlCommandType::Retry {
                            task_id: current_task,
                            iteration: pane.state.current_iteration,
                        },
                    ) {
                        ConductorAction::error(format!("Retry failed: {}", e))
                    } else {
                        ConductorAction::success("Retry command sent")
                    }
                } else {
                    ConductorAction::warning("No active task to retry")
                }
            } else {
                ConductorAction::warning("No track active")
            }
        }
        (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
            // Skip current task
            if let Some(track_id) = pane.state.current_track.clone() {
                if let Some(current_task) = pane.state.current_task.clone() {
                    if let Err(e) = send_control_command(
                        &track_id,
                        ControlCommandType::Skip {
                            task_id: current_task,
                            iteration: pane.state.current_iteration,
                        },
                    ) {
                        ConductorAction::error(format!("Skip failed: {}", e))
                    } else {
                        ConductorAction::success("Skip command sent")
                    }
                } else {
                    ConductorAction::warning("No active task to skip")
                }
            } else {
                ConductorAction::warning("No track active")
            }
        }
        (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
            // Abort orchestrate session
            if let Some(track_id) = pane.state.current_track.clone() {
                if let Err(e) = send_control_command(
                    &track_id,
                    ControlCommandType::Abort {
                        reason: Some("User requested abort via conductor".to_string()),
                    },
                ) {
                    ConductorAction::error(format!("Abort failed: {}", e))
                } else {
                    ConductorAction::success("Abort command sent")
                }
            } else {
                ConductorAction::warning("No track active")
            }
        }

        // Execution control (Ralph-style shortcuts)
        (_, KeyCode::Char('s')) => {
            let track_idx = match pane.get_selected_track_index() {
                Some(idx) => idx,
                None => return ConductorAction::warning("No track selected"),
            };
            let track_id = &pane.tracks[track_idx].id;

            // Prevention: Don't start if already running
            if pane.state.track_runtime_statuses.get(track_id)
                == Some(&crate::conductor::model::ConductorStatus::Running)
            {
                return ConductorAction::warning(format!("Track {} is already running", track_id));
            }

            let cmd = pane.get_start_command(None, false, false);
            match cmd {
                Some(cmd) => {
                    if let Err(e) = execute_orchestrate_command(&cmd) {
                        return ConductorAction::error(format!("Error: {}", e));
                    }
                    ConductorAction::success(format!("Started: {}", cmd))
                }
                None => ConductorAction::warning("No track selected"),
            }
        }
        // Note: We handle CONTROL+s for skip separately below, so check for it first
        // For pause, only match without modifiers (or with SHIFT which is same key)
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char('p')) => {
            let cmd = pane.get_pause_command();
            match cmd {
                Some(cmd) => {
                    if let Err(e) = execute_orchestrate_command(&cmd) {
                        return ConductorAction::error(format!("Error: {}", e));
                    }
                    ConductorAction::success(format!("Paused: {}", cmd))
                }
                None => ConductorAction::warning("No track selected"),
            }
        }
        // Note: We handle CONTROL+r for retry separately below, so check for it first
        // For resume, only match without modifiers
        (KeyModifiers::NONE, KeyCode::Char('r')) => {
            let cmd = pane.get_resume_command();
            match cmd {
                Some(cmd) => {
                    if let Err(e) = execute_orchestrate_command(&cmd) {
                        return ConductorAction::error(format!("Error: {}", e));
                    }
                    ConductorAction::success(format!("Resumed: {}", cmd))
                }
                None => ConductorAction::warning("No track selected"),
            }
        }
        (_, KeyCode::Char('?')) => {
            let cmd = pane.get_status_command();
            if let Err(e) = execute_orchestrate_command(&cmd) {
                return ConductorAction::error(format!("Error: {}", e));
            }
            ConductorAction::info(format!("Status checked: {}", cmd))
        }

        _ => ConductorAction::None,
    }
}

/// Execute an orchestrate command using type-safe CommandArgs.
/// This avoids shell injection vulnerabilities by using proper argument separation.
fn execute_orchestrate_command(cmd: &CommandArgs) -> std::io::Result<std::process::Child> {
    cmd.spawn_detached()
}

/// Control command types for Ralph behavior enforcement
enum ControlCommandType {
    Retry {
        task_id: String,
        iteration: u64,
    },
    Skip {
        task_id: String,
        iteration: u64,
    },
    Abort {
        reason: Option<String>,
    },
    Steer {
        message: String,
    },
    SetMaxIterations {
        max: u64,
    },
    SetErrorStrategy {
        strategy: String,
    },
    SwitchAgent {
        tool: String,
        _model: Option<String>,
    },
    SetLoopMode {
        mode: String,
    },
}

/// Send a control command to the orchestrate engine via control.json
fn send_control_command(track_id: &str, cmd: ControlCommandType) -> anyhow::Result<()> {
    use leindex_core::orchestrate::control::{ControlCommand, ControlFile};
    use std::fs;

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let control_path = std::path::PathBuf::from(home)
        .join(".maestro")
        .join("orchestrate")
        .join(track_id)
        .join("control.json");

    // Read existing control file or create new one
    let mut control = if control_path.exists() {
        let content = fs::read_to_string(&control_path)?;
        serde_json::from_str::<ControlFile>(&content)?
    } else {
        ControlFile::default()
    };

    // Convert our command to the orchestrate control command
    let orch_cmd = match cmd {
        ControlCommandType::Retry { task_id, iteration } => {
            ControlCommand::Retry { task_id, iteration }
        }
        ControlCommandType::Skip { task_id, iteration } => {
            ControlCommand::Skip { task_id, iteration }
        }
        ControlCommandType::Abort { reason } => ControlCommand::Abort { reason },
        ControlCommandType::Steer { message } => ControlCommand::Steer(message),
        ControlCommandType::SetMaxIterations { max } => {
            ControlCommand::SetMaxIterations(max as usize)
        }
        ControlCommandType::SetErrorStrategy { strategy } => {
            use leindex_core::orchestrate::control::ErrorStrategyValue;
            let strategy_value = match strategy.as_str() {
                "retry" => ErrorStrategyValue::Retry,
                "skip" => ErrorStrategyValue::Skip,
                "abort" => ErrorStrategyValue::Abort,
                _ => ErrorStrategyValue::Retry, // Default to retry
            };
            ControlCommand::SetErrorStrategy {
                strategy: strategy_value,
            }
        }
        ControlCommandType::SwitchAgent { tool, _model: _ } => {
            // Note: model is ignored for now as ControlCommand::SwitchAgent only takes tool
            ControlCommand::SwitchAgent(tool)
        }
        ControlCommandType::SetLoopMode { mode } => {
            use leindex_core::orchestrate::model::LoopMode;
            let loop_mode_value = match mode.as_str() {
                "planning" => LoopMode::Planning,
                "building" => LoopMode::Building,
                _ => LoopMode::Building, // Default to Building
            };
            ControlCommand::SetLoopMode(loop_mode_value)
        }
    };

    // Add command to the control file
    control.commands.push(orch_cmd);
    control.updated_at = chrono::Utc::now().to_rfc3339();

    // Write back atomically
    let temp_path = control_path.with_extension("tmp");
    let content = serde_json::to_string_pretty(&control)?;
    fs::write(&temp_path, content)?;
    fs::rename(&temp_path, &control_path)?;

    Ok(())
}
