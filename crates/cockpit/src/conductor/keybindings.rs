use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::conductor::pane::ConductorPane;

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
                    pane.state.selected_project_index = (pane.state.selected_project_index + 1) % pane.state.available_projects.len();
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
                    let project = pane.state.available_projects[pane.state.selected_project_index].clone();
                    pane.switch_project(project);
                }
                return ConductorAction::Handled;
            }
            _ => return ConductorAction::Handled, // Absorb other keys when modal is open
        }
    }

    // 2. Handle Dashboard if open
    if pane.show_dashboard {
        if key.code == KeyCode::Esc || key.code == KeyCode::Char('d') {
            pane.show_dashboard = false;
            return ConductorAction::Handled;
        }
        return ConductorAction::Handled;
    }

    match (key.modifiers, key.code) {
        // Navigation
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            if pane.output_focused && pane.details_mode == crate::conductor::model::DetailsViewMode::Output {
                pane.scroll_output_up(); 
            } else {
                pane.move_selection(1);
            }
            ConductorAction::Handled
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            if pane.output_focused && pane.details_mode == crate::conductor::model::DetailsViewMode::Output {
                pane.scroll_output_down();
            } else {
                pane.move_selection(-1);
            }
            ConductorAction::Handled
        }
        (KeyModifiers::NONE, KeyCode::PageDown) => {
            if pane.output_focused && pane.details_mode == crate::conductor::model::DetailsViewMode::Output {
                for _ in 0..10 { pane.scroll_output_up(); }
            }
            ConductorAction::Handled
        }
        (KeyModifiers::NONE, KeyCode::PageUp) => {
            if pane.output_focused && pane.details_mode == crate::conductor::model::DetailsViewMode::Output {
                for _ in 0..10 { pane.scroll_output_down(); }
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

        // Details mode switching
        (_, KeyCode::Char('1')) => {
            pane.details_mode = crate::conductor::model::DetailsViewMode::Details;
            ConductorAction::Handled
        }
        (_, KeyCode::Char('2')) => {
            pane.details_mode = crate::conductor::model::DetailsViewMode::Output;
            ConductorAction::Handled
        }
        (_, KeyCode::Char('3')) => {
            pane.details_mode = crate::conductor::model::DetailsViewMode::Prompt;
            ConductorAction::Handled
        }

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
        (KeyModifiers::SHIFT, KeyCode::Char('P')) | (KeyModifiers::ALT, KeyCode::Char('p')) if !pane.output_focused => {
            pane.open_project_selector();
            ConductorAction::Handled
        }

        // Focus toggle
        (KeyModifiers::NONE, KeyCode::Tab) | (KeyModifiers::ALT, KeyCode::Char('p')) => {
            pane.output_focused = !pane.output_focused;
            ConductorAction::Handled
        }

        // Execution control (Ralph-style shortcuts)
        (_, KeyCode::Char('s')) => {
            let track_idx = match pane.get_selected_track_index() {
                Some(idx) => idx,
                None => return ConductorAction::StatusMessage("No track selected".to_string()),
            };
            let track_id = &pane.tracks[track_idx].id;
            
            // Prevention: Don't start if already running
            if pane.state.track_runtime_statuses.get(track_id) == Some(&crate::conductor::model::ConductorStatus::Running) {
                return ConductorAction::StatusMessage(format!("Track {} is already running", track_id));
            }

            let cmd = pane.get_start_command(None, false, false);
            if let Err(e) = execute_orchestrate_command(&cmd) {
                 return ConductorAction::StatusMessage(format!("Error: {}", e));
            }
            ConductorAction::StatusMessage(format!("Started: {}", cmd))
        }
        (_, KeyCode::Char('p')) => {
            let cmd = pane.get_pause_command();
            if let Err(e) = execute_orchestrate_command(&cmd) {
                 return ConductorAction::StatusMessage(format!("Error: {}", e));
            }
            ConductorAction::StatusMessage(format!("Paused: {}", cmd))
        }
        (_, KeyCode::Char('r')) => {
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
    let parts: Vec<String> = cmd.split(" --").enumerate().map(|(i, s)| {
        if i == 0 {
            s.to_string()
        } else {
            format!("--{}", s)
        }
    }).collect();

    if parts.is_empty() {
        return Ok(());
    }

    // First part contains "maestro orchestrate <subcmd> <track_id>"
    let mut initial_parts: Vec<&str> = parts[0].split_whitespace().collect();
    if initial_parts.is_empty() { return Ok(()); }

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
    std::process::Command::new(program)
        .args(args)
        .spawn()?;
    
    Ok(())
}
