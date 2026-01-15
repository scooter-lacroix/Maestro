use std::io;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph},
    Frame, Terminal,
};

mod setup;
use setup::{run_orchestra, SetupEvent};

struct App {
    phase: Phase,
    frame_count: u64,
    should_quit: bool,
    install_progress: f64,
    current_action: String,
    logs: Vec<String>,
    receiver: Option<Receiver<SetupEvent>>,
    error: Option<String>,
    // Config options
    install_path: String,
    editor: String,
    // Granular tool selection
    tool_selections: Vec<(String, bool)>,
    config_selection: usize, // 0: path, 1: editor, 2: tools..., N: star, N+1: confirm
    starred: bool,
}

#[derive(PartialEq, Clone, Copy)]
enum Phase {
    Overture,    // Welcome
    Tuning,      // Configuration
    Performance, // Installation
    Crescendo,   // Compilation (Special display)
    Ovation,     // Complete
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App {
        phase: Phase::Overture,
        frame_count: 0,
        should_quit: false,
        install_progress: 0.0,
        current_action: "Arranging the orchestra...".to_string(),
        logs: vec!["Welcome to Maestro Setup v2.0".to_string()],
        receiver: None,
        error: None,
        install_path: "~/.maestro".to_string(),
        editor: "hx".to_string(),
        tool_selections: vec![
            ("Go Language (for Zoekt)".to_string(), true),
            ("Zoekt (Fast Code Search)".to_string(), true),
            ("Tmux / Tmux-RS".to_string(), true),
            ("Yazi (Terminal File Manager)".to_string(), true),
            ("Claude Code (by Anthropic)".to_string(), true),
            ("Gemini CLI (by Google)".to_string(), true),
            ("Codex CLI (OpenAI)".to_string(), true),
            ("OpenCode (Independent)".to_string(), true),
            ("Amp (by Sourcegraph)".to_string(), true),
        ],
        config_selection: 0,
        starred: true,
    };

    let res = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let tick_rate = Duration::from_millis(50);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Some(ref rx) = app.receiver {
            while let Ok(event) = rx.try_recv() {
                match event {
                    SetupEvent::ActionStarted(msg) => {
                        app.current_action = msg.clone();
                        app.logs.push(format!("CONDUCTOR: {}", msg));
                        if msg.contains("Compiling") {
                            app.phase = Phase::Crescendo;
                        } else {
                            app.phase = Phase::Performance;
                        }
                    }
                    SetupEvent::StepCompleted(current, total) => {
                        app.install_progress = (current as f64 / total as f64) * 100.0;
                    }
                    SetupEvent::Log(msg) => {
                        app.logs.push(msg);
                    }
                    SetupEvent::Finished => {
                        app.phase = Phase::Ovation;
                        app.install_progress = 100.0;
                    }
                    SetupEvent::Error(msg) => {
                        app.error = Some(msg);
                    }
                }
            }
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Enter => {
                        if app.phase == Phase::Overture {
                            app.phase = Phase::Tuning;
                        } else if app.phase == Phase::Tuning {
                            let total_options = 3 + app.tool_selections.len() + 1; // Path, Editor, Star, Tools, Confirm
                            if app.config_selection == total_options - 1 {
                                let config = setup::Config {
                                    install_path: app.install_path.clone(),
                                    editor: app.editor.clone(),
                                    selected_tools: app
                                        .tool_selections
                                        .iter()
                                        .filter(|(_, s)| *s)
                                        .map(|(n, _)| n.clone())
                                        .collect(),
                                };
                                let (tx, rx) = mpsc::channel();
                                app.receiver = Some(rx);
                                thread::spawn(move || {
                                    run_orchestra(tx, config);
                                });
                                app.phase = Phase::Performance;
                            } else {
                                app.config_selection = (app.config_selection + 1) % total_options;
                            }
                        } else if app.phase == Phase::Ovation {
                            app.should_quit = true;
                        }
                    }
                    KeyCode::Up => {
                        if app.phase == Phase::Tuning {
                            let total_options = 3 + app.tool_selections.len() + 1;
                            app.config_selection = if app.config_selection == 0 {
                                total_options - 1
                            } else {
                                app.config_selection - 1
                            };
                        }
                    }
                    KeyCode::Down => {
                        if app.phase == Phase::Tuning {
                            let total_options = 3 + app.tool_selections.len() + 1;
                            app.config_selection = (app.config_selection + 1) % total_options;
                        }
                    }
                    KeyCode::Char(' ') | KeyCode::Right => {
                        if app.phase == Phase::Tuning {
                            match app.config_selection {
                                1 => {
                                    let editors = ["hx", "nvim", "vim", "code", "fresh"];
                                    let current_idx =
                                        editors.iter().position(|&e| e == app.editor).unwrap_or(0);
                                    app.editor =
                                        editors[(current_idx + 1) % editors.len()].to_string();
                                }
                                idx if idx >= 2 && idx < 2 + app.tool_selections.len() => {
                                    let tool_idx = idx - 2;
                                    app.tool_selections[tool_idx].1 =
                                        !app.tool_selections[tool_idx].1;
                                }
                                idx if idx == 2 + app.tool_selections.len() => {
                                    app.starred = !app.starred;
                                }
                                _ => {}
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        if app.phase == Phase::Tuning && app.config_selection == 0 {
                            app.install_path.pop();
                        }
                    }
                    KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.should_quit = true;
                    }
                    KeyCode::Char(c) => {
                        if app.phase == Phase::Tuning && app.config_selection == 0 {
                            app.install_path.push(c);
                        } else if c == 'q' {
                            app.should_quit = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.frame_count = app.frame_count.wrapping_add(1);
            last_tick = Instant::now();
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let bg_style = Style::default().bg(Color::Rgb(10, 10, 18));
    f.render_widget(Block::default().style(bg_style), size);

    match app.phase {
        Phase::Overture => render_overture(f, app, size),
        Phase::Tuning => render_tuning(f, app, size),
        Phase::Performance => render_performance(f, app, size),
        Phase::Crescendo => render_crescendo(f, app, size),
        Phase::Ovation => render_ovation(f, app, size),
    }

    if let Some(ref err) = app.error {
        render_error_modal(f, err, size);
    }
}

fn render_overture(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(area);

    let anim_char = match (app.frame_count / 10) % 4 {
        0 => "⠋",
        1 => "⠙",
        2 => "⠹",
        _ => "⠸",
    };

    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                format!(" {}  ", anim_char),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "MAESTRO CONDUCTOR WIZARD",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {} ", anim_char),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![Span::styled(
            "v2.0 Unified Installer",
            Style::default().fg(Color::DarkGray),
        )]),
    ])
    .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    let welcome_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(15, 15, 25)));

    let welcome_text = vec![
        Line::from("Welcome to the Grand Performance."),
        Line::from(""),
        Line::from("Every masterpiece needs a conductor, and every conductor needs a score."),
        Line::from("I will scan your environment and harmonize your system dependencies."),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled(
                "[ ENTER ]",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to begin the Overture."),
        ]),
    ];

    let p = Paragraph::new(welcome_text)
        .alignment(Alignment::Center)
        .block(welcome_block);
    f.render_widget(p, centered_rect(70, 40, chunks[1]));
}

fn render_tuning(f: &mut Frame, app: &mut App, area: Rect) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new("🛠 THE TUNING PHASE 🛠")
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        areas[0],
    );

    let mut items = vec![
        ListItem::new(vec![
            Line::from("  Installation Path:"),
            Line::from(vec![Span::styled(
                format!("    > {}", app.install_path),
                if app.config_selection == 0 {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            )]),
        ]),
        ListItem::new(vec![
            Line::from("  Preferred Editor:"),
            Line::from(vec![Span::styled(
                format!(
                    "    ← {} → (Press Space/Right to cycle)",
                    app.editor.to_uppercase()
                ),
                if app.config_selection == 1 {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            )]),
        ]),
        ListItem::new(Line::from("  Include Tooling (Space to toggle):")),
    ];

    // Add granular tool selections
    for (idx, (name, selected)) in app.tool_selections.iter().enumerate() {
        let sel_idx = 2 + idx;
        let checkbox = if *selected { " [X] " } else { " [ ] " };
        items.push(ListItem::new(Line::from(vec![Span::styled(
            format!("    {}{}", checkbox, name),
            if app.config_selection == sel_idx {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            },
        )])));
    }

    // Star the repo item
    let star_idx = 2 + app.tool_selections.len();
    let star_check = if app.starred { " ⭐ " } else { " ☆ " };
    items.push(ListItem::new(vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("  {} Star the Maestro on GitHub", star_check),
            if app.config_selection == star_idx {
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            },
        )]),
    ]));

    // Confirmation button
    let confirm_idx = 3 + app.tool_selections.len();
    items.push(ListItem::new(vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  [ STRIKE THE FIRST CHORD (Launch Installation) ]",
            if app.config_selection == confirm_idx {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            },
        )]),
    ]));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" System Configuration ")
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, centered_rect(70, 70, areas[1]));

    let footer = Paragraph::new(vec![Line::from(vec![
        Span::styled(" ↑/↓: Navigate ", Style::default().fg(Color::DarkGray)),
        Span::styled(" • ", Style::default().fg(Color::DarkGray)),
        Span::styled(" Space/→: Toggle ", Style::default().fg(Color::DarkGray)),
        Span::styled(" • ", Style::default().fg(Color::DarkGray)),
        Span::styled(" Enter: Confirm ", Style::default().fg(Color::DarkGray)),
        Span::styled(" • ", Style::default().fg(Color::DarkGray)),
        Span::styled(" Ctrl+Q: Quit ", Style::default().fg(Color::Red)),
    ])])
    .alignment(Alignment::Center);
    f.render_widget(footer, areas[2]);
}

fn render_performance(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    let anim_frames = ["♪", "♫", "♬", "♩"];
    let anim = anim_frames[(app.frame_count / 15 % 4) as usize];

    f.render_widget(
        Paragraph::new(format!("{} The Performance is in Progress {}", anim, anim))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan)),
        chunks[0],
    );

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" Progress "))
        .gauge_style(
            Style::default()
                .fg(Color::Green)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .percent(app.install_progress as u16);
    f.render_widget(gauge, chunks[1]);

    let logs: Vec<ListItem> = app
        .logs
        .iter()
        .rev()
        .take(10)
        .map(|l| ListItem::new(l.as_str()))
        .collect();
    let log_list = List::new(logs).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Conductor's Notes "),
    );
    f.render_widget(log_list, chunks[2]);
}

fn render_crescendo(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(10),
            Constraint::Min(0),
        ])
        .split(area);

    let pulse = [" ", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃"];
    let pulse_str = pulse[(app.frame_count % 12) as usize];

    let header = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "⚡ THE CRESCENDO ⚡",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("Compiling the Grand Orchestra (Maestro Core)"),
    ])
    .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    let visual = Paragraph::new(format!(
        "{}{}{}{}{}{}{}{}{}{}",
        pulse_str,
        pulse_str,
        pulse_str,
        pulse_str,
        pulse_str,
        pulse_str,
        pulse_str,
        pulse_str,
        pulse_str,
        pulse_str
    ))
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::Rgb(200, 100, 255)));
    f.render_widget(visual, chunks[1]);

    let quirky_phrases = [
        "Brass section entering with intensity...",
        "Polishing the violins for the final solo...",
        "Tuning the harps for a perfect finish...",
        "Wait, is that a rogue oboe? Fixed it.",
        "The percussion is hitting just right.",
        "Almost at the grand finale!",
    ];
    let phrase = quirky_phrases[(app.frame_count / 100 % 6) as usize];

    let p = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            phrase,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::ITALIC),
        )]),
        Line::from(""),
        Line::from(format!("Current Progress: {:.1}%", app.install_progress)),
    ])
    .alignment(Alignment::Center);
    f.render_widget(p, chunks[2]);
}

fn render_ovation(f: &mut Frame, _app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Min(0),
            Constraint::Length(5),
        ])
        .split(area);

    let congrat = Paragraph::new("✨ STANDING OVATION ✨")
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    f.render_widget(congrat, chunks[0]);

    let summary = vec![
        Line::from("The performance was a masterpiece."),
        Line::from(""),
        Line::from(vec![
            Span::styled("→ System Harmony: ", Style::default().fg(Color::Cyan)),
            Span::raw("Achieved"),
        ]),
        Line::from(vec![
            Span::styled("→ Maestro Core: ", Style::default().fg(Color::Cyan)),
            Span::raw("Compiled & Ready"),
        ]),
        Line::from(vec![
            Span::styled("→ Zoekt Search: ", Style::default().fg(Color::Cyan)),
            Span::raw("Operational"),
        ]),
        Line::from(""),
        Line::from("You may now begin conducting your projects."),
    ];
    let p = Paragraph::new(summary).alignment(Alignment::Center);
    f.render_widget(p, chunks[1]);

    let command = Paragraph::new(vec![
        Line::from("To start your cockpit:"),
        Line::from(vec![Span::styled(
            "  maestro  ",
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )]),
    ])
    .alignment(Alignment::Center);
    f.render_widget(command, chunks[2]);
}

fn render_error_modal(f: &mut Frame, msg: &str, area: Rect) {
    let modal_area = centered_rect(60, 20, area);
    f.render_widget(Clear, modal_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ❌ DISTORTION IN THE SCORE ")
        .border_style(Style::default().fg(Color::Red));
    let p = Paragraph::new(msg)
        .block(block)
        .style(Style::default().fg(Color::Red))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(p, modal_area);
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
