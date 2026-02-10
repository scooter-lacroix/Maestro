use crate::conductor::pane::ConductorPane;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Result of a key event handling
pub enum ConductorAction {
    /// Event was not handled by Conductor
    None,
    /// Event was handled, no further action needed
    Handled,
    /// Event was handled, display this status message
    StatusMessage(String),
}

/// Handle Conductor-specific key events
pub fn handle_key_event(pane: &mut ConductorPane, key: KeyEvent) -> ConductorAction {
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

        // Details mode switching (Legacy fallback removed to allow tab switching)

        // Output control
        (_, KeyCode::Char('c')) => {
            pane.clear_output();
            ConductorAction::Handled
        }

        // Dashboard toggle
        (_, KeyCode::Char('d')) => {
            pane.show_dashboard = !pane.show_dashboard;
            ConductorAction::Handled
        }

        // Project Selector
        (KeyModifiers::SHIFT, KeyCode::Char('P')) => {
            pane.open_project_selector();
            ConductorAction::Handled
        }

        // Focus toggle
        (KeyModifiers::ALT, KeyCode::Char('p')) => {
            pane.output_focused = !pane.output_focused;
            let msg = if pane.output_focused {
                "Output focused. Scroll with Arrows/PgUp/PgDn."
            } else {
                "Tracks focused."
            };
            ConductorAction::StatusMessage(msg.to_string())
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
                        ConductorAction::StatusMessage(format!("Retry failed: {}", e))
                    } else {
                        ConductorAction::StatusMessage("Retry command sent".to_string())
                    }
                } else {
                    ConductorAction::StatusMessage("No active task to retry".to_string())
                }
            } else {
                ConductorAction::StatusMessage("No track active".to_string())
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
                        ConductorAction::StatusMessage(format!("Skip failed: {}", e))
                    } else {
                        ConductorAction::StatusMessage("Skip command sent".to_string())
                    }
                } else {
                    ConductorAction::StatusMessage("No active task to skip".to_string())
                }
            } else {
                ConductorAction::StatusMessage("No track active".to_string())
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
                    ConductorAction::StatusMessage(format!("Abort failed: {}", e))
                } else {
                    ConductorAction::StatusMessage("Abort command sent".to_string())
                }
            } else {
                ConductorAction::StatusMessage("No track active".to_string())
            }
        }

        // Execution control (Ralph-style shortcuts)
        (_, KeyCode::Char('s')) => {
            let track_idx = match pane.get_selected_track_index() {
                Some(idx) => idx,
                None => return ConductorAction::StatusMessage("No track selected".to_string()),
            };
            let track_id = &pane.tracks[track_idx].id;

            // Prevention: Don't start if already running
            if pane.state.track_runtime_statuses.get(track_id)
                == Some(&crate::conductor::model::ConductorStatus::Running)
            {
                return ConductorAction::StatusMessage(format!(
                    "Track {} is already running",
                    track_id
                ));
            }

            let cmd = pane.get_start_command(None, false, false);
            if let Err(e) = execute_orchestrate_command(&cmd) {
                return ConductorAction::StatusMessage(format!("Error: {}", e));
            }
            ConductorAction::StatusMessage(format!("Started: {}", cmd))
        }
        // Note: We handle CONTROL+s for skip separately below, so check for it first
        // For pause, only match without modifiers (or with SHIFT which is same key)
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char('p')) => {
            let cmd = pane.get_pause_command();
            if let Err(e) = execute_orchestrate_command(&cmd) {
                return ConductorAction::StatusMessage(format!("Error: {}", e));
            }
            ConductorAction::StatusMessage(format!("Paused: {}", cmd))
        }
        // Note: We handle CONTROL+r for retry separately below, so check for it first
        // For resume, only match without modifiers
        (KeyModifiers::NONE, KeyCode::Char('r')) => {
            let cmd = pane.get_resume_command();
            if let Err(e) = execute_orchestrate_command(&cmd) {
                return ConductorAction::StatusMessage(format!("Error: {}", e));
            }
            ConductorAction::StatusMessage(format!("Resumed: {}", cmd))
        }
        (_, KeyCode::Char('?')) => {
            let cmd = pane.get_status_command();
            if let Err(e) = execute_orchestrate_command(&cmd) {
                return ConductorAction::StatusMessage(format!("Error: {}", e));
            }
            ConductorAction::StatusMessage(format!("Status checked: {}", cmd))
        }

        _ => ConductorAction::None,
    }
}

fn execute_orchestrate_command(cmd: &str) -> std::io::Result<()> {
    // Robust argument parsing: handle spaces in paths/IDs using shell-like splitting
    // For simplicity here, we'll use a basic vector of args.
    let parts: Vec<String> = cmd
        .split(" --")
        .enumerate()
        .map(|(i, s)| {
            if i == 0 {
                s.to_string()
            } else {
                format!("--{}", s)
            }
        })
        .collect();

    if parts.is_empty() {
        return Ok(());
    }

    // First part contains "maestro orchestrate <subcmd> <track_id>"
    let initial_parts: Vec<&str> = parts[0].split_whitespace().collect();
    if initial_parts.is_empty() {
        return Ok(());
    }

    let program_name = initial_parts[0];
    let program = if program_name == "maestro" {
        std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("maestro"))
    } else {
        std::path::PathBuf::from(program_name)
    };

    let mut args = Vec::new();
    // Add "orchestrate", "<subcmd>", "<track_id>"
    if initial_parts.len() > 1 {
        args.extend(initial_parts[1..].iter().map(|s| s.to_string()));
    }

    // Add the remaining flags
    for part in &parts[1..] {
        let flag_parts: Vec<&str> = part.split_whitespace().collect();
        for fp in flag_parts {
            args.push(fp.to_string());
        }
    }

    // Run it detached
    std::process::Command::new(program).args(args).spawn()?;

    Ok(())
}

/// Control command types for Ralph behavior enforcement
enum ControlCommandType {
    Retry { task_id: String, iteration: u64 },
    Skip { task_id: String, iteration: u64 },
    Abort { reason: Option<String> },
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
