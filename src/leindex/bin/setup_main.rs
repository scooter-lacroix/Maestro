use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use leindex_core::setup::password::PasswordCache;
use leindex_core::setup::{
    detect_distro, run_orchestra, Config, Distro, SetupEvent, StepDescriptor,
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};

#[derive(Debug, Clone, PartialEq)]
pub enum PasswordState {
    None,
    Prompting { service: String, prompt: String },
}

struct App {
    phase: Phase,
    frame_count: u64,
    should_quit: bool,
    install_progress: f64,
    current_action: String,
    logs: Vec<LogEntry>,
    steps: Vec<InstallStep>,
    active_step: Option<usize>,
    completed_steps: usize,
    receiver: Option<Receiver<SetupEvent>>,
    error: Option<SetupFailure>,
    password_state: PasswordState,
    password_buffer: String,
    distro: Distro,
    install_path: String,
    editor: String,
    leindex_install_method: String,
    nexus_install_method: String,
    tool_selections: Vec<(String, bool)>,
    config_selection: usize,
    starred: bool,
    password_cache: Arc<PasswordCache>,
    install_log_path: PathBuf,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Phase {
    Overture,
    Tuning,
    Installing,
    Ovation,
    Failed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InstallStepState {
    Pending,
    Running,
    Completed,
    Failed,
}

struct InstallStep {
    name: String,
    description: String,
    state: InstallStepState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LogLevel {
    Info,
    Output,
    Success,
    Warning,
    Error,
}

struct LogEntry {
    level: LogLevel,
    message: String,
}

struct SetupFailure {
    step: Option<String>,
    message: String,
    hint: Option<String>,
}

impl LogEntry {
    fn info(message: impl Into<String>) -> Self {
        Self {
            level: LogLevel::Info,
            message: message.into(),
        }
    }

    fn from_setup_log(message: impl Into<String>) -> Self {
        let message = message.into();
        let trimmed = message.trim();
        let level = if trimmed.starts_with("[ERR]") {
            LogLevel::Error
        } else if trimmed.starts_with("[WARN]") || trimmed.starts_with("[sudo]") {
            LogLevel::Warning
        } else if trimmed.starts_with("[OK]") {
            LogLevel::Success
        } else if trimmed.starts_with("[OUT]") {
            LogLevel::Output
        } else {
            LogLevel::Info
        };

        Self { level, message }
    }
}

fn resolve_install_log_path() -> PathBuf {
    resolve_install_log_path_with(|key| std::env::var(key).ok())
}

fn resolve_install_log_path_with<F>(mut lookup: F) -> PathBuf
where
    F: FnMut(&str) -> Option<String>,
{
    for key in [
        "MAESTRO_SETUP_LOG_FILE",
        "MAESTRO_INSTALL_LOG_FILE",
        "MAESTRO_SETUP_LOG",
        "MAESTRO_INSTALL_LOG",
    ] {
        if let Some(path) = lookup(key) {
            return PathBuf::from(path);
        }
    }

    let mut dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push(".maestro");
    dir.push("logs");
    let _ = fs::create_dir_all(&dir);
    dir.join(format!(
        "install-{}.log",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    ))
}

fn append_install_log(path: &Path, line: impl AsRef<str>) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}", line.as_ref());
    }
}

fn display_install_log_path(path: &Path) -> String {
    path.display().to_string()
}

fn main() -> Result<(), io::Error> {
    // Check for headless mode
    let headless = std::env::args().any(|arg| arg == "--headless")
        || std::env::var("MAESTRO_HEADLESS")
            .map(|v| v == "1" || v == "true")
            .unwrap_or(false);

    if headless {
        return run_headless_install();
    }

    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        eprintln!("Error: Maestro Setup Wizard requires an interactive terminal.");
        eprintln!();
        eprintln!("This installer uses a terminal UI (TUI) that requires:");
        eprintln!("  - A proper TTY (terminal) attached to stdin/stdout");
        eprintln!("  - An interactive shell session");
        eprintln!();
        eprintln!("If you're seeing this error, try:");
        eprintln!("  1. Run the installer directly in your terminal (not via a script redirect)");
        eprintln!("  2. Make sure you're not piping input/output");
        eprintln!("  3. Try: bash install.sh (from your terminal)");
        eprintln!(
            "  4. For headless/CI installs, use: cargo run --bin maestro-setup -- --headless"
        );
        eprintln!();
        std::process::exit(1);
    }

    let install_log_path = resolve_install_log_path();
    append_install_log(
        &install_log_path,
        format!(
            "[{}] Interactive Maestro setup wizard started",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ),
    );
    println!("Detailed install log: {}", install_log_path.display());

    let mut stdout = io::stdout();
    enable_raw_mode().map_err(|e| {
        eprintln!("Failed to enable raw mode: {}", e);
        e
    })?;

    if let Err(e) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
        let _ = disable_raw_mode();
        eprintln!("Failed to enter alternate screen: {}", e);
        return Err(e);
    }

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).inspect_err(|_| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    })?;

    let detected_distro = detect_distro();
    append_install_log(
        &install_log_path,
        format!(
            "[{}] Maestro setup wizard started",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ),
    );
    let app = App {
        phase: Phase::Overture,
        frame_count: 0,
        should_quit: false,
        install_progress: 0.0,
        current_action: "Review the score before we start.".to_string(),
        logs: vec![
            LogEntry::info("Welcome to Maestro Setup"),
            LogEntry::info(format!(
                "Detected environment: {} ({})",
                detected_distro,
                detected_distro.package_manager_name()
            )),
            LogEntry::info("Choose what to install, then launch the conductor."),
        ],
        steps: Vec::new(),
        active_step: None,
        completed_steps: 0,
        receiver: None,
        error: None,
        password_state: PasswordState::None,
        password_buffer: String::new(),
        distro: detected_distro,
        install_path: "~/.maestro".to_string(),
        editor: "hx".to_string(),
        leindex_install_method: std::env::var("MAESTRO_LEINDEX_INSTALL_METHOD")
            .unwrap_or_else(|_| "cargo".to_string()),
        nexus_install_method: std::env::var("MAESTRO_NEXUS_INSTALL_METHOD")
            .unwrap_or_else(|_| "git".to_string()),
        tool_selections: vec![
            ("Go Language (for Zoekt)".to_string(), true),
            ("Zoekt (Fast Code Search)".to_string(), true),
            ("Tmux / Tmux-RS".to_string(), true),
            ("Yazi (Terminal File Manager)".to_string(), true),
            ("Claude Code (by Anthropic)".to_string(), true),
            ("Gemini CLI (by Google)".to_string(), true),
            ("iFlow CLI (by iFlow)".to_string(), true),
            ("Qwen Code (QwenLM)".to_string(), true),
            ("Codex CLI (OpenAI)".to_string(), true),
            ("OpenCode (Independent)".to_string(), true),
            ("Amp CLI (by Sourcegraph)".to_string(), true),
            ("Droid CLI (by Factory)".to_string(), true),
            ("pi-mono (Multi-Model CLI)".to_string(), true),
        ],
        config_selection: 0,
        starred: true,
        password_cache: Arc::new(PasswordCache::new()),
        install_log_path: install_log_path.clone(),
    };

    let res = run_app(&mut terminal, app);
    cleanup_terminal(&mut terminal)?;

    match res {
        Ok(Phase::Ovation) => {}
        Ok(phase) => {
            append_install_log(
                &install_log_path,
                format!("Setup wizard exited before successful completion (phase: {:?})", phase),
            );
            std::process::exit(1);
        }
        Err(err) => {
            println!("{:?}", err);
            return Err(err);
        }
    }

    Ok(())
}

fn cleanup_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), io::Error> {
    execute!(terminal.backend_mut(), DisableMouseCapture)?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    disable_raw_mode()?;
    std::thread::sleep(Duration::from_millis(50));
    Ok(())
}

/// Run installation in headless mode (no TUI) for CI/automation
fn run_headless_install() -> Result<(), io::Error> {
    use std::sync::mpsc;
    use std::thread;

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║       Maestro Setup Wizard (Headless Mode)               ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    let detected_distro = detect_distro();
    println!(
        "Detected environment: {} ({})",
        detected_distro,
        detected_distro.package_manager_name()
    );

    let install_log_path = resolve_install_log_path();
    append_install_log(
        &install_log_path,
        format!(
            "[{}] Headless Maestro setup started",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ),
    );
    println!("Detailed install log: {}", install_log_path.display());

    // Default configuration for headless mode
    let install_path =
        std::env::var("MAESTRO_INSTALL_PATH").unwrap_or_else(|_| "~/.maestro".to_string());
    let editor = std::env::var("MAESTRO_EDITOR").unwrap_or_else(|_| "hx".to_string());
    let leindex_install_method =
        std::env::var("MAESTRO_LEINDEX_INSTALL_METHOD").unwrap_or_else(|_| "cargo".to_string());
    let nexus_install_method =
        std::env::var("MAESTRO_NEXUS_INSTALL_METHOD").unwrap_or_else(|_| "git".to_string());

    // Get tool selections from environment or use defaults
    let mut selected_tools = Vec::new();
    let all_tools = vec![
        ("Go Language (for Zoekt)", "MAESTRO_INSTALL_GO"),
        ("Zoekt (Fast Code Search)", "MAESTRO_INSTALL_ZOEKT"),
        ("Tmux / Tmux-RS", "MAESTRO_INSTALL_TMUX"),
        ("Yazi (Terminal File Manager)", "MAESTRO_INSTALL_YAZI"),
        ("Claude Code (by Anthropic)", "MAESTRO_INSTALL_CLAUDE"),
        ("Gemini CLI (by Google)", "MAESTRO_INSTALL_GEMINI"),
        ("iFlow CLI (by iFlow)", "MAESTRO_INSTALL_IFLOW"),
        ("Qwen Code (QwenLM)", "MAESTRO_INSTALL_QWEN"),
        ("Codex CLI (OpenAI)", "MAESTRO_INSTALL_CODEX"),
        ("OpenCode (Independent)", "MAESTRO_INSTALL_OPENCODE"),
        ("Amp CLI (by Sourcegraph)", "MAESTRO_INSTALL_AMP"),
        ("Droid CLI (by Factory)", "MAESTRO_INSTALL_DROID"),
        ("pi-mono (Multi-Model CLI)", "MAESTRO_INSTALL_PIMONO"),
    ];

    // In headless mode, install all tools by default unless explicitly disabled
    for (tool_name, env_var) in &all_tools {
        let should_install = match std::env::var(env_var) {
            Ok(v) => !matches!(v.to_lowercase().as_str(), "0" | "false" | "no"),
            Err(_) => true, // Default to installing if env var not set
        };
        if should_install {
            selected_tools.push(tool_name.to_string());
        }
    }

    println!("Install path: {}", install_path);
    println!("Editor: {}", editor);
    println!("Selected components: {}", selected_tools.len());
    for tool in &selected_tools {
        println!("  - {}", tool);
    }
    println!();

    // Create config and run orchestra
    let config = Config {
        install_path,
        editor,
        selected_tools,
        leindex_install_method,
        nexus_install_method,
        password_cache: Arc::new(PasswordCache::new()),
        distro: detected_distro,
    };

    let (tx, rx) = mpsc::channel();

    // Spawn orchestra in a thread
    thread::spawn(move || {
        run_orchestra(tx, config);
    });

    // Process events and print to stdout
    let mut current_step = 0;
    let mut total_steps = 0;
    let mut failed = false;

    loop {
        match rx.recv() {
            Ok(event) => {
                match event {
                    SetupEvent::PlanReady(plan) => {
                        total_steps = plan.len();
                        append_install_log(
                            &install_log_path,
                            format!("Prepared installation plan with {} step(s)", total_steps),
                        );
                        println!("Installation plan: {} steps", total_steps);
                        println!();
                    }
                    SetupEvent::StepStarted {
                        current,
                        total,
                        step,
                    } => {
                        current_step = current;
                        append_install_log(
                            &install_log_path,
                            format!(
                                "START [{}/{}] {} — {}",
                                current, total, step.name, step.description
                            ),
                        );
                        print!("[{}/{}] {}... ", current, total, step.name);
                        io::stdout().flush()?;
                    }
                    SetupEvent::StepCompleted {
                        current,
                        total,
                        step_name: _,
                    } => {
                        append_install_log(
                            &install_log_path,
                            format!("DONE [{}/{}]", current, total),
                        );
                        println!("✓");
                        if current == total {
                            println!();
                            println!("═══════════════════════════════════════════════════════════");
                            println!("Installation completed successfully!");
                            println!("═══════════════════════════════════════════════════════════");
                            println!();
                            println!("Next command: maestro");
                        }
                    }
                    SetupEvent::Log(msg) => {
                        append_install_log(&install_log_path, msg.trim_end());
                        // Only print important logs in headless mode
                        let trimmed = msg.trim();
                        if trimmed.starts_with("[ERR]")
                            || trimmed.starts_with("[WARN]")
                            || trimmed.starts_with("Error:")
                            || trimmed.starts_with("Warning:")
                        {
                            println!();
                            println!("  → {}", trimmed);
                            print!("[{}/{}] Continuing... ", current_step, total_steps);
                            io::stdout().flush()?;
                        }
                    }
                    SetupEvent::Error {
                        step,
                        message,
                        hint,
                    } => {
                        failed = true;
                        append_install_log(
                            &install_log_path,
                            format!("ERROR step={:?} message={} hint={:?}", step, message, hint),
                        );
                        println!();
                        println!();
                        println!("═══════════════════════════════════════════════════════════");
                        println!("INSTALLATION FAILED");
                        println!("═══════════════════════════════════════════════════════════");
                        if let Some(s) = step {
                            println!("Failed step: {}", s);
                        }
                        println!("Error: {}", message);
                        if let Some(h) = hint {
                            println!("Hint: {}", h);
                        }
                        println!();
                    }
                    SetupEvent::Finished => {
                        append_install_log(&install_log_path, "Installation finished");
                        if !failed {
                            println!();
                            println!("═══════════════════════════════════════════════════════════");
                            println!("Installation completed successfully!");
                            println!("═══════════════════════════════════════════════════════════");
                            println!();
                            println!("Next command: maestro");
                        }
                        break;
                    }
                    _ => {}
                }
            }
            Err(_) => {
                println!("Installation channel closed unexpectedly.");
                break;
            }
        }
    }

    if failed {
        std::process::exit(1);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<Phase> {
    let tick_rate = Duration::from_millis(50);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        loop {
            let next_event = match app.receiver.as_ref() {
                Some(rx) => rx.try_recv().ok(),
                None => None,
            };

            let Some(event) = next_event else {
                break;
            };

            match event {
                SetupEvent::PlanReady(plan) => {
                    append_install_log(
                        &app.install_log_path,
                        format!("Prepared installation plan with {} step(s)", plan.len()),
                    );
                    app.steps = plan
                        .into_iter()
                        .map(|step| InstallStep {
                            name: step.name,
                            description: step.description,
                            state: InstallStepState::Pending,
                        })
                        .collect();
                    app.logs.push(LogEntry::info(format!(
                        "Prepared {} installation step(s).",
                        app.steps.len()
                    )));
                }
                SetupEvent::StepStarted {
                    current,
                    total,
                    step,
                } => {
                    append_install_log(
                        &app.install_log_path,
                        format!(
                            "START [{}/{}] {} — {}",
                            current, total, step.name, step.description
                        ),
                    );
                    app.phase = Phase::Installing;
                    app.current_action = step.description.clone();
                    app.active_step = Some(current.saturating_sub(1));
                    app.install_progress = if total == 0 {
                        0.0
                    } else {
                        ((current.saturating_sub(1)) as f64 / total as f64) * 100.0
                    };
                    sync_step_plan(&mut app, current, total, &step);
                    app.logs.push(LogEntry::info(format!(
                        "Starting step {}/{}: {}",
                        current, total, step.name
                    )));
                }
                SetupEvent::StepCompleted {
                    current,
                    total,
                    step_name,
                } => {
                    append_install_log(
                        &app.install_log_path,
                        format!("DONE [{}/{}] {}", current, total, step_name),
                    );
                    app.completed_steps = current;
                    app.install_progress = if total == 0 {
                        100.0
                    } else {
                        (current as f64 / total as f64) * 100.0
                    };
                    if let Some(step) = app.steps.get_mut(current.saturating_sub(1)) {
                        step.state = InstallStepState::Completed;
                    }
                    app.active_step = None;
                    app.current_action = format!("Completed {}", step_name);
                    app.logs.push(LogEntry::info(format!(
                        "Completed step {}/{}.",
                        current, total
                    )));
                }
                SetupEvent::PasswordPrompt { service, prompt } => {
                    app.password_state = PasswordState::Prompting {
                        service,
                        prompt: prompt.clone(),
                    };
                    app.password_buffer.clear();
                    app.logs.push(LogEntry {
                        level: LogLevel::Warning,
                        message: prompt,
                    });
                }
                SetupEvent::Log(msg) => {
                    append_install_log(&app.install_log_path, msg.trim_end());
                    app.logs.push(LogEntry::from_setup_log(msg));
                    if app.logs.len() > 250 {
                        app.logs.remove(0);
                    }
                }
                SetupEvent::Finished => {
                    append_install_log(&app.install_log_path, "Installation finished");
                    app.phase = Phase::Ovation;
                    app.install_progress = 100.0;
                    app.password_state = PasswordState::None;
                    app.current_action = "Installation completed.".to_string();
                    app.completed_steps = app.steps.len();
                    for step in &mut app.steps {
                        if step.state == InstallStepState::Running {
                            step.state = InstallStepState::Completed;
                        }
                    }
                }
                SetupEvent::Error {
                    step,
                    message,
                    hint,
                } => {
                    append_install_log(
                        &app.install_log_path,
                        format!("ERROR step={:?} message={} hint={:?}", step, message, hint),
                    );
                    if let Some(active) = app.active_step {
                        if let Some(step) = app.steps.get_mut(active) {
                            step.state = InstallStepState::Failed;
                        }
                    }
                    if let Some(step_name) = &step {
                        if let Some(index) =
                            app.steps.iter().position(|entry| &entry.name == step_name)
                        {
                            app.steps[index].state = InstallStepState::Failed;
                        }
                    }
                    app.phase = Phase::Failed;
                    app.password_state = PasswordState::None;
                    app.error = Some(SetupFailure {
                        step,
                        message: message.clone(),
                        hint,
                    });
                    app.logs.push(LogEntry {
                        level: LogLevel::Error,
                        message,
                    });
                }
            }
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if matches!(app.password_state, PasswordState::Prompting { .. }) {
                    match key.code {
                        KeyCode::Enter => {
                            let password = app.password_buffer.clone();
                            app.logs.push(LogEntry::info(format!(
                                "Password received ({} character(s)).",
                                password.len()
                            )));
                            app.password_cache.set_password(password);
                            app.password_state = PasswordState::None;
                            app.password_buffer.clear();
                        }
                        KeyCode::Backspace => {
                            app.password_buffer.pop();
                        }
                        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.should_quit = true;
                        }
                        KeyCode::Char(c) => app.password_buffer.push(c),
                        _ => {}
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Enter => match app.phase {
                        Phase::Overture => app.phase = Phase::Tuning,
                        Phase::Tuning => {
                            let total_options = 6 + app.tool_selections.len();
                            if app.config_selection == total_options - 1 {
                                let config = Config {
                                    install_path: app.install_path.clone(),
                                    editor: app.editor.clone(),
                                    selected_tools: app
                                        .tool_selections
                                        .iter()
                                        .filter(|(_, selected)| *selected)
                                        .map(|(name, _)| name.clone())
                                        .collect(),
                                    leindex_install_method: app.leindex_install_method.clone(),
                                    nexus_install_method: app.nexus_install_method.clone(),
                                    password_cache: Arc::clone(&app.password_cache),
                                    distro: app.distro,
                                };

                                let (tx, rx) = mpsc::channel();
                                app.receiver = Some(rx);
                                app.phase = Phase::Installing;
                                app.error = None;
                                app.logs.push(LogEntry::info(
                                    "Launching the conductor and generating the installation plan.",
                                ));
                                thread::spawn(move || {
                                    run_orchestra(tx, config);
                                });
                            } else {
                                app.config_selection = (app.config_selection + 1) % total_options;
                            }
                        }
                        Phase::Ovation | Phase::Failed => app.should_quit = true,
                        Phase::Installing => {}
                    },
                    KeyCode::Up => {
                        if app.phase == Phase::Tuning {
                            let total_options = 6 + app.tool_selections.len();
                            app.config_selection = if app.config_selection == 0 {
                                total_options - 1
                            } else {
                                app.config_selection - 1
                            };
                        }
                    }
                    KeyCode::Down => {
                        if app.phase == Phase::Tuning {
                            let total_options = 6 + app.tool_selections.len();
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
                                2 => {
                                    let methods = ["cargo", "install-script", "pypi", "skip"];
                                    let current_idx = methods
                                        .iter()
                                        .position(|&m| m == app.leindex_install_method)
                                        .unwrap_or(0);
                                    app.leindex_install_method =
                                        methods[(current_idx + 1) % methods.len()].to_string();
                                }
                                3 => {
                                    let methods = ["git", "cargo", "skip"];
                                    let current_idx = methods
                                        .iter()
                                        .position(|&m| m == app.nexus_install_method)
                                        .unwrap_or(0);
                                    app.nexus_install_method =
                                        methods[(current_idx + 1) % methods.len()].to_string();
                                }
                                idx if idx >= 4 && idx < 4 + app.tool_selections.len() => {
                                    let tool_idx = idx - 4;
                                    app.tool_selections[tool_idx].1 =
                                        !app.tool_selections[tool_idx].1;
                                }
                                idx if idx == 4 + app.tool_selections.len() => {
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
            return Ok(app.phase);
        }
    }
}

fn sync_step_plan(app: &mut App, current: usize, total: usize, active_step: &StepDescriptor) {
    if app.steps.is_empty() {
        app.steps = (0..total)
            .map(|_| InstallStep {
                name: "Pending".to_string(),
                description: "Waiting to start...".to_string(),
                state: InstallStepState::Pending,
            })
            .collect();
    }

    while app.steps.len() < total {
        app.steps.push(InstallStep {
            name: "Pending".to_string(),
            description: "Waiting to start...".to_string(),
            state: InstallStepState::Pending,
        });
    }

    for (idx, step) in app.steps.iter_mut().enumerate() {
        step.state = if idx < app.completed_steps {
            InstallStepState::Completed
        } else if idx == current.saturating_sub(1) {
            InstallStepState::Running
        } else if step.state == InstallStepState::Failed {
            InstallStepState::Failed
        } else {
            InstallStepState::Pending
        };
    }

    if let Some(step) = app.steps.get_mut(current.saturating_sub(1)) {
        step.name = active_step.name.clone();
        step.description = active_step.description.clone();
        step.state = InstallStepState::Running;
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(9, 12, 18))),
        size,
    );

    match app.phase {
        Phase::Overture => render_overture(f, app, size),
        Phase::Tuning => render_tuning(f, app, size),
        Phase::Installing => render_installing(f, app, size),
        Phase::Ovation => render_ovation(f, app, size),
        Phase::Failed => render_failure(f, app, size),
    }

    if matches!(app.password_state, PasswordState::Prompting { .. }) {
        render_password_modal(f, app, size);
    }
}

fn render_password_modal(f: &mut Frame, app: &mut App, area: Rect) {
    let modal_area = centered_rect(52, 28, area);
    f.render_widget(Clear, modal_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Password Required ")
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Rgb(20, 24, 32)));

    let (requester, prompt) = match &app.password_state {
        PasswordState::Prompting { service, prompt } => (service.as_str(), prompt.as_str()),
        PasswordState::None => ("system", "Administrator privileges are required."),
    };
    let masked = "*".repeat(app.password_buffer.len());
    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Secure action requested by ",
                Style::default().fg(Color::Gray),
            ),
            Span::styled(requester, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", prompt),
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Password: ", Style::default().fg(Color::Yellow)),
            Span::styled(masked, Style::default().fg(Color::White)),
            Span::styled("▌", Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(Color::Green)),
            Span::raw(" submit  "),
            Span::styled("Ctrl+Q", Style::default().fg(Color::Red)),
            Span::raw(" exit installer"),
        ]),
    ];
    f.render_widget(Paragraph::new(text).block(block), modal_area);
}

fn render_overture(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(0),
            Constraint::Length(4),
        ])
        .split(area);

    let pulse = match (app.frame_count / 10) % 4 {
        0 => "⠋",
        1 => "⠙",
        2 => "⠹",
        _ => "⠸",
    };

    let hero = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(format!(" {} ", pulse), Style::default().fg(Color::Yellow)),
            Span::styled(
                "MAESTRO CONDUCTOR WIZARD",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" {} ", pulse), Style::default().fg(Color::Yellow)),
        ]),
        Line::from(Span::styled(
            "Interactive installer for the current Maestro build",
            Style::default().fg(Color::Gray),
        )),
    ])
    .alignment(Alignment::Center);
    f.render_widget(hero, chunks[0]);

    let body_area = centered_rect(72, 56, chunks[1]);
    let body = Paragraph::new(vec![
        Line::from("The overture sets the stage before anything changes on disk."),
        Line::from(""),
        Line::from("What you can expect:"),
        Line::from("  • clear component selection"),
        Line::from("  • step-by-step installation progress"),
        Line::from("  • live logs and explicit failure messages"),
        Line::from(""),
        Line::from(vec![
            Span::raw("Environment: "),
            Span::styled(
                format!("{} ({})", app.distro, app.distro.package_manager_name()),
                Style::default().fg(Color::Magenta),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::Gray)),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::styled(" to configure the score.", Style::default().fg(Color::Gray)),
        ]),
    ])
    .alignment(Alignment::Left)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Before We Begin ")
            .style(Style::default().bg(Color::Rgb(16, 20, 28))),
    )
    .wrap(Wrap { trim: true });
    f.render_widget(body, body_area);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("Ctrl+Q", Style::default().fg(Color::Red)),
        Span::styled(" quits at any time", Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}

fn render_tuning(f: &mut Frame, app: &mut App, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            "Installation Plan Builder",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "Choose your install path, preferred editor, and which components belong in this run.",
            Style::default().fg(Color::Gray),
        )),
    ]);
    f.render_widget(header, sections[0]);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(sections[1]);

    let summary = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Install path: ", Style::default().fg(Color::Gray)),
            Span::styled(&app.install_path, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Editor: ", Style::default().fg(Color::Gray)),
            Span::styled(app.editor.to_uppercase(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("LeIndex install: ", Style::default().fg(Color::Gray)),
            Span::styled(
                app.leindex_install_method.to_uppercase(),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("Nexus install: ", Style::default().fg(Color::Gray)),
            Span::styled(
                app.nexus_install_method.to_uppercase(),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("Environment: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{} ({})", app.distro, app.distro.package_manager_name()),
                Style::default().fg(Color::Magenta),
            ),
        ]),
        Line::from(vec![
            Span::styled("Selected components: ", Style::default().fg(Color::Gray)),
            Span::styled(
                app.tool_selections
                    .iter()
                    .filter(|(_, selected)| *selected)
                    .count()
                    .to_string(),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                format!(" of {}", app.tool_selections.len()),
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(""),
        Line::from("The installer will build a concrete step plan from these choices."),
        Line::from("The themed phrases stay, but the progress screen will show real output."),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Run Summary "),
    )
    .wrap(Wrap { trim: true });
    f.render_widget(summary, columns[0]);

    let mut items = vec![
        ListItem::new(vec![
            Line::from(Span::styled(
                "Install path",
                if app.config_selection == 0 {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            )),
            Line::from(Span::styled(
                format!("  > {}", app.install_path),
                if app.config_selection == 0 {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Gray)
                },
            )),
        ]),
        ListItem::new(vec![
            Line::from(Span::styled(
                "Preferred editor",
                if app.config_selection == 1 {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            )),
            Line::from(Span::styled(
                format!("  ← {} →", app.editor.to_uppercase()),
                if app.config_selection == 1 {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Gray)
                },
            )),
        ]),
        ListItem::new(vec![
            Line::from(Span::styled(
                "LeIndex install method",
                if app.config_selection == 2 {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            )),
            Line::from(Span::styled(
                format!("  ← {} →", app.leindex_install_method.to_uppercase()),
                if app.config_selection == 2 {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Gray)
                },
            )),
        ]),
        ListItem::new(vec![
            Line::from(Span::styled(
                "Nexus install method",
                if app.config_selection == 3 {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            )),
            Line::from(Span::styled(
                format!("  ← {} →", app.nexus_install_method.to_uppercase()),
                if app.config_selection == 3 {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Gray)
                },
            )),
        ]),
        ListItem::new(Line::from(Span::styled(
            "Components",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))),
    ];

    for (idx, (name, selected)) in app.tool_selections.iter().enumerate() {
        let is_selected = *selected;
        let is_active = app.config_selection == idx + 4;
        let marker = if is_selected { "[x]" } else { "[ ]" };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("  {} ", marker),
                if is_selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
            Span::styled(
                name,
                if is_active {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            ),
        ])));
    }

    let star_idx = 4 + app.tool_selections.len();
    items.push(ListItem::new(Line::from(vec![
        Span::styled(
            if app.starred { "  [★] " } else { "  [ ] " },
            if app.starred {
                Style::default().fg(Color::Magenta)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
        Span::styled(
            "Star Maestro on GitHub",
            if app.config_selection == star_idx {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            },
        ),
    ])));

    let launch_idx = star_idx + 1;
    items.push(ListItem::new(Line::from(vec![Span::styled(
        "  Launch installation",
        if app.config_selection == launch_idx {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        },
    )])));

    let config_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Configuration ")
            .style(Style::default().bg(Color::Rgb(14, 18, 26))),
    );
    f.render_widget(config_list, columns[1]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("↑/↓", Style::default().fg(Color::Gray)),
        Span::styled(" move  ", Style::default().fg(Color::Gray)),
        Span::styled("Space/→", Style::default().fg(Color::Gray)),
        Span::styled(" toggle or cycle  ", Style::default().fg(Color::Gray)),
        Span::styled("Enter", Style::default().fg(Color::Green)),
        Span::styled(" continue  ", Style::default().fg(Color::Gray)),
        Span::styled("Ctrl+Q", Style::default().fg(Color::Red)),
        Span::styled(" quit", Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(footer, sections[2]);
}

fn render_installing(f: &mut Frame, app: &mut App, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(6),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            "Installer Dashboard",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            current_flavor_line(app),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::ITALIC),
        )),
    ]);
    f.render_widget(header, sections[0]);

    let summary_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(3)])
        .split(sections[1]);
    let total_steps = app.steps.len().max(app.completed_steps.max(1));
    let current_step_label = if let Some(active) = app.active_step {
        format!("Step {} of {}", active + 1, total_steps)
    } else if app.completed_steps == total_steps {
        format!("All {} steps completed", total_steps)
    } else {
        format!("{} of {} steps completed", app.completed_steps, total_steps)
    };
    let summary = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(current_step_label, Style::default().fg(Color::Green)),
            Span::styled("  •  ", Style::default().fg(Color::DarkGray)),
            Span::styled(&app.current_action, Style::default().fg(Color::White)),
        ]),
        Line::from(Span::styled(
            format!("Progress: {:.1}%", app.install_progress),
            Style::default().fg(Color::Gray),
        )),
    ]);
    f.render_widget(summary, summary_chunks[0]);

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Overall Progress "),
        )
        .gauge_style(
            Style::default()
                .fg(Color::Green)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .percent(app.install_progress.round().clamp(0.0, 100.0) as u16);
    f.render_widget(gauge, summary_chunks[1]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(sections[2]);

    let step_items: Vec<ListItem> = app
        .steps
        .iter()
        .enumerate()
        .map(|(idx, step)| {
            let (marker, style) = match step.state {
                InstallStepState::Pending => ("○", Style::default().fg(Color::DarkGray)),
                InstallStepState::Running => (
                    "▶",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                InstallStepState::Completed => ("✓", Style::default().fg(Color::Green)),
                InstallStepState::Failed => ("✗", Style::default().fg(Color::Red)),
            };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!(" {} ", marker), style),
                    Span::styled(
                        &step.name,
                        if Some(idx) == app.active_step {
                            style
                        } else {
                            Style::default().fg(Color::White)
                        },
                    ),
                ]),
                Line::from(Span::styled(
                    format!("   {}", step.description),
                    Style::default().fg(Color::Gray),
                )),
            ])
        })
        .collect();
    let step_list = List::new(step_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Step Plan "),
    );
    f.render_widget(step_list, body[0]);

    let log_items: Vec<ListItem> = app
        .logs
        .iter()
        .rev()
        .take(18)
        .map(|entry| {
            let style = match entry.level {
                LogLevel::Info => Style::default().fg(Color::White),
                LogLevel::Output => Style::default().fg(Color::Gray),
                LogLevel::Success => Style::default().fg(Color::Green),
                LogLevel::Warning => Style::default().fg(Color::Yellow),
                LogLevel::Error => Style::default().fg(Color::Red),
            };
            ListItem::new(Line::from(Span::styled(&entry.message, style)))
        })
        .collect();
    let logs = List::new(log_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Live Output "),
    );
    f.render_widget(logs, body[1]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            "Themed status lines are decorative only. ",
            Style::default().fg(Color::Gray),
        ),
        Span::styled("Live Output", Style::default().fg(Color::Cyan)),
        Span::styled(
            " shows the real command stream.",
            Style::default().fg(Color::Gray),
        ),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(footer, sections[3]);
}

fn render_ovation(f: &mut Frame, app: &mut App, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(0),
            Constraint::Length(5),
        ])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            "Standing Ovation",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "Every requested installation step finished successfully.",
            Style::default().fg(Color::Gray),
        )),
    ])
    .alignment(Alignment::Center);
    f.render_widget(header, sections[0]);

    let summary = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Completed steps: ", Style::default().fg(Color::Gray)),
            Span::styled(
                app.completed_steps.to_string(),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("Selected components: ", Style::default().fg(Color::Gray)),
            Span::styled(
                app.tool_selections
                    .iter()
                    .filter(|(_, selected)| *selected)
                    .count()
                    .to_string(),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(""),
        Line::from("Next command:"),
        Line::from(Span::styled(
            "  maestro",
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Press Enter to leave the installer."),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Summary "),
    );
    f.render_widget(summary, centered_rect(64, 56, sections[1]));
}

fn render_failure(f: &mut Frame, app: &mut App, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(0),
            Constraint::Length(4),
        ])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            "The Score Broke",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "Installation stopped before completion. The logs below contain the exact failure details.",
            Style::default().fg(Color::Gray),
        )),
    ]);
    f.render_widget(header, sections[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(sections[1]);

    let steps = List::new(
        app.steps
            .iter()
            .map(|step| {
                let marker = match step.state {
                    InstallStepState::Completed => "✓",
                    InstallStepState::Running => "▶",
                    InstallStepState::Failed => "✗",
                    InstallStepState::Pending => "○",
                };
                let style = match step.state {
                    InstallStepState::Completed => Style::default().fg(Color::Green),
                    InstallStepState::Running => Style::default().fg(Color::Cyan),
                    InstallStepState::Failed => Style::default().fg(Color::Red),
                    InstallStepState::Pending => Style::default().fg(Color::DarkGray),
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", marker), style),
                    Span::styled(&step.name, Style::default().fg(Color::White)),
                ]))
            })
            .collect::<Vec<_>>(),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Step Status "),
    );
    f.render_widget(steps, body[0]);

    let failure_right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(0)])
        .split(body[1]);

    let failure_lines = if let Some(error) = &app.error {
        let mut lines = vec![
            Line::from(Span::styled(
                &error.message,
                Style::default().fg(Color::Red),
            )),
            Line::from(""),
        ];
        if let Some(step) = &error.step {
            lines.push(Line::from(vec![
                Span::styled("Failed step: ", Style::default().fg(Color::Gray)),
                Span::styled(step, Style::default().fg(Color::White)),
            ]));
        }
        if let Some(hint) = &error.hint {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Hint: ", Style::default().fg(Color::Yellow)),
                Span::styled(hint, Style::default().fg(Color::Gray)),
            ]));
        }
        // Always show the durable log path so users can debug after exiting the TUI.
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Full log: ", Style::default().fg(Color::Gray)),
            Span::styled(
                display_install_log_path(&app.install_log_path),
                Style::default().fg(Color::Cyan),
            ),
        ]));
        lines
    } else {
        vec![Line::from(Span::styled(
            "Installation failed without an explicit error message.",
            Style::default().fg(Color::Red),
        ))]
    };

    let error_text = Paragraph::new(failure_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Failure Details "),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(error_text, failure_right[0]);

    let log_items: Vec<ListItem> = app
        .logs
        .iter()
        .rev()
        .take(12)
        .map(|entry| {
            let style = match entry.level {
                LogLevel::Info => Style::default().fg(Color::White),
                LogLevel::Output => Style::default().fg(Color::Gray),
                LogLevel::Success => Style::default().fg(Color::Green),
                LogLevel::Warning => Style::default().fg(Color::Yellow),
                LogLevel::Error => Style::default().fg(Color::Red),
            };
            ListItem::new(Line::from(Span::styled(&entry.message, style)))
        })
        .collect();
    let logs = List::new(log_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Live Output "),
    );
    f.render_widget(logs, failure_right[1]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            "Review the failure details above, then press ",
            Style::default().fg(Color::Gray),
        ),
        Span::styled("Enter", Style::default().fg(Color::Green)),
        Span::styled(" to exit.", Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(footer, sections[2]);
}

fn current_flavor_line(app: &App) -> &'static str {
    if app.error.is_some() {
        "The brass section missed a cue. We are holding for recovery."
    } else if app.install_progress >= 90.0 {
        "Final cadence. The house lights are already warming."
    } else if app.current_action.contains("Compiling") || app.current_action.contains("build") {
        "Percussion is carrying the build while the strings hold tempo."
    } else if app.install_progress >= 50.0 {
        "Mid-performance. The orchestra is settling into the groove."
    } else {
        "The overture is under way. Every section is joining in order."
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_explicit_setup_log_file_over_legacy_env() {
        let path = resolve_install_log_path_with(|key| match key {
            "MAESTRO_SETUP_LOG_FILE" => Some("/tmp/setup.log".to_string()),
            "MAESTRO_INSTALL_LOG" => Some("/tmp/install.log".to_string()),
            _ => None,
        });

        assert_eq!(path, PathBuf::from("/tmp/setup.log"));
    }

    #[test]
    fn falls_back_to_install_log_env_when_setup_log_missing() {
        let path = resolve_install_log_path_with(|key| match key {
            "MAESTRO_INSTALL_LOG_FILE" => Some("/tmp/install.log".to_string()),
            _ => None,
        });

        assert_eq!(path, PathBuf::from("/tmp/install.log"));
    }
}
