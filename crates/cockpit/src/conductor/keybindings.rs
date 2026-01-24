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
        // Tree navigation
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            pane.move_selection(1);
            ConductorAction::Handled
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            pane.move_selection(-1);
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
        (KeyModifiers::NONE, KeyCode::Char(' ')) | (KeyModifiers::NONE, KeyCode::Enter) => {
            if !pane.output_focused {
                let items = pane.get_selectable_items();
                if let Some(item) = items.get(pane.selected_index) {
                    let id = match item {
                        crate::conductor::model::SelectableItem::Track { id, .. } => id.clone(),
                        crate::conductor::model::SelectableItem::Task { id, .. } => id.clone(),
                    };
                    pane.toggle_task_expansion(&id);
                }
                ConductorAction::Handled
            } else {
                ConductorAction::None
            }
        }

        // Details mode switching
        (KeyModifiers::NONE, KeyCode::Char('1')) => {
            pane.details_mode = crate::conductor::model::DetailsViewMode::Details;
            ConductorAction::Handled
        }
        (KeyModifiers::NONE, KeyCode::Char('2')) => {
            pane.details_mode = crate::conductor::model::DetailsViewMode::Output;
            ConductorAction::Handled
        }
        (KeyModifiers::NONE, KeyCode::Char('3')) => {
            pane.details_mode = crate::conductor::model::DetailsViewMode::Prompt;
            ConductorAction::Handled
        }

        // Output control
        (KeyModifiers::NONE, KeyCode::Char('c')) => {
            pane.clear_output();
            ConductorAction::Handled
        }

        // Dashboard toggle
        (KeyModifiers::NONE, KeyCode::Char('d')) => {
            pane.show_dashboard = !pane.show_dashboard;
            ConductorAction::Handled
        }

        // Project Selector
        (KeyModifiers::SHIFT, KeyCode::Char('P')) => {
            pane.open_project_selector();
            ConductorAction::Handled
        }

        // Execution control (Ralph-style shortcuts)
        (KeyModifiers::NONE, KeyCode::Char('s')) => {
            let cmd = pane.get_start_command(None, false, false);
            ConductorAction::StatusMessage(format!("Start: {}", cmd))
        }
        (KeyModifiers::NONE, KeyCode::Char('p')) => {
            let cmd = pane.get_pause_command();
            ConductorAction::StatusMessage(format!("Pause: {}", cmd))
        }
        (KeyModifiers::NONE, KeyCode::Char('r')) => {
            let cmd = pane.get_resume_command();
            ConductorAction::StatusMessage(format!("Resume: {}", cmd))
        }
        (KeyModifiers::NONE, KeyCode::Char('?')) => {
            let cmd = pane.get_status_command();
            ConductorAction::StatusMessage(format!("Status: {}", cmd))
        }

        // Focus toggle
        (KeyModifiers::NONE, KeyCode::Tab) => {
            pane.output_focused = !pane.output_focused;
            ConductorAction::Handled
        }

        _ => ConductorAction::None,
    }
}
