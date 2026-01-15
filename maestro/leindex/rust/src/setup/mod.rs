use std::process::Command;
use std::sync::mpsc::Sender;

pub enum SetupEvent {
    ActionStarted(String),
    StepCompleted(usize, usize), // current, total
    Log(String),
    Finished,
    Error(String),
}

pub struct Step {
    pub name: String,
    pub description: String,
    pub command: String,
}

pub struct Config {
    pub install_path: String,
    pub editor: String,
    pub include_tooling: bool,
}

pub fn run_orchestra(tx: Sender<SetupEvent>, config: Config) {
    let mut steps = vec![
        Step {
            name: "The Overture".to_string(),
            description: format!("Preparing stage at {}...", config.install_path),
            command: format!("mkdir -p {} && sleep 1", config.install_path),
        },
        Step {
            name: "Woodwinds".to_string(),
            description: "Installing basic utilities (curl, unzip, build-essential)...".to_string(),
            command: "sudo apt-get update && sudo apt-get install -y curl unzip build-essential pkg-config libssl-dev".to_string(),
        },
    ];

    if config.include_tooling {
        steps.push(Step {
            name: "Brass Section".to_string(),
            description: "Synchronizing Go environment...".to_string(),
            command: "sudo apt-get install -y golang-go".to_string(),
        });
        steps.push(Step {
            name: "Percussion".to_string(),
            description: "Setting up Tmux...".to_string(),
            command: "sudo apt-get install -y tmux".to_string(),
        });
    }

    steps.extend(vec![
        Step {
            name: "Bass Note".to_string(),
            description: "Ensuring Yazi (File Picker) is present...".to_string(),
            command: "command -v yazi > /dev/null 2>&1 || sudo apt-get install -y yazi > /dev/null 2>&1 || cargo install --locked yazi-fm yazi-cli".to_string(),
        },
        Step {
            name: "Conductor's Baton".to_string(),
            description: format!("Setting default editor to {}...", config.editor),
            command: "sleep 0.5".to_string(), // Placeholder for config write
        },
        Step {
            name: "The Crescendo".to_string(),
            description: "Compiling the Maestro Core...".to_string(),
            command: "cargo build --release".to_string(),
        },
    ]);

    let total = steps.len();
    for (i, step) in steps.into_iter().enumerate() {
        let _ = tx.send(SetupEvent::ActionStarted(step.description));
        let _ = tx.send(SetupEvent::Log(format!(
            "CONDUCTOR: Commencing {}",
            step.name
        )));

        // Execute command and capture output to prevent UI corruption
        let output = Command::new("bash")
            .arg("-c")
            .arg(&step.command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output();

        match output {
            Ok(out) => {
                // Send logs from stdout/stderr to the TUI instead of terminal
                if !out.stdout.is_empty() {
                    let s = String::from_utf8_lossy(&out.stdout);
                    for line in s.lines().take(5) {
                        // Only take a few lines to avoid flooding
                        let _ = tx.send(SetupEvent::Log(format!("  [OUT] {}", line)));
                    }
                }
                if !out.stderr.is_empty() {
                    let s = String::from_utf8_lossy(&out.stderr);
                    for line in s.lines().take(5) {
                        let _ = tx.send(SetupEvent::Log(format!("  [ERR] {}", line)));
                    }
                }

                if out.status.success() {
                    let _ = tx.send(SetupEvent::StepCompleted(i + 1, total));
                } else {
                    let _ = tx.send(SetupEvent::Error(format!(
                        "Step '{}' failed with exit code: {}",
                        step.name, out.status
                    )));
                    return;
                }
            }
            Err(e) => {
                let _ = tx.send(SetupEvent::Error(format!(
                    "Failed to execute step '{}': {}",
                    step.name, e
                )));
                return;
            }
        }
    }

    let _ = tx.send(SetupEvent::Finished);
}
