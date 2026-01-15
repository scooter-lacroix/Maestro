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

pub fn run_orchestra(tx: Sender<SetupEvent>) {
    let steps = vec![
        Step {
            name: "The Overture".to_string(),
            description: "Checking system acoustics (dependencies)".to_string(),
            command: "sleep 1".to_string(),
        },
        Step {
            name: "Woodwinds".to_string(),
            description: "Installing basic utilities (curl, unzip, build-essential)...".to_string(),
            command: "sudo apt-get update && sudo apt-get install -y curl unzip build-essential pkg-config libssl-dev > /dev/null 2>&1 || true".to_string(),
        },
        Step {
            name: "Brass Section".to_string(),
            description: "Synchronizing Go environment...".to_string(),
            command: "sudo apt-get install -y golang-go > /dev/null 2>&1 || true".to_string(),
        },
        Step {
            name: "Percussion".to_string(),
            description: "Setting up Tmux...".to_string(),
            command: "sudo apt-get install -y tmux > /dev/null 2>&1 || true".to_string(),
        },
        Step {
            name: "Bass Note".to_string(),
            description: "Ensuring Yazi (File Picker) is present...".to_string(),
            command: "command -v yazi > /dev/null 2>&1 || sudo apt-get install -y yazi > /dev/null 2>&1 || cargo install --locked yazi-fm yazi-cli > /dev/null 2>&1 || true".to_string(),
        },
        Step {
            name: "The Crescendo".to_string(),
            description: "Compiling the Maestro Core...".to_string(),
            command: "cargo build --release".to_string(),
        },
    ];

    let total = steps.len();
    for (i, step) in steps.into_iter().enumerate() {
        let _ = tx.send(SetupEvent::ActionStarted(step.description));
        let _ = tx.send(SetupEvent::Log(format!("Executing: {}", step.name)));

        // Execute command
        let status = Command::new("bash").arg("-c").arg(&step.command).status();

        match status {
            Ok(s) if s.success() => {
                let _ = tx.send(SetupEvent::StepCompleted(i + 1, total));
            }
            Ok(s) => {
                let _ = tx.send(SetupEvent::Error(format!(
                    "Step '{}' failed with exit code: {}",
                    step.name, s
                )));
                return;
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
