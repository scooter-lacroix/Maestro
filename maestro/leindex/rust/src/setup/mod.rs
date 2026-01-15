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
    pub selected_tools: Vec<String>,
}

pub fn run_orchestra(tx: Sender<SetupEvent>, config: Config) {
    let mut steps = Vec::new();

    steps.push(Step {
        name: "The Overture".to_string(),
        description: format!("Preparing stage at {}...", config.install_path),
        command: format!("mkdir -p {} && sleep 1", config.install_path),
    });

    steps.push(Step {
        name: "Woodwinds".to_string(),
        description: "Installing basic utilities (curl, unzip, build-essential)...".to_string(),
        command: "sudo apt-get update && sudo apt-get install -y curl unzip build-essential pkg-config libssl-dev".to_string(),
    });

    if config.editor == "fresh" {
        steps.push(Step {
            name: "The Fresh Script".to_string(),
            description: "Installing FRESH (https://github.com/sinelaw/fresh)...".to_string(),
            command: "curl https://raw.githubusercontent.com/sinelaw/fresh/refs/heads/master/scripts/install.sh | sh".to_string(),
        });
    }

    // Handle Tooling Granularly
    for tool in &config.selected_tools {
        match tool.as_str() {
            "Go Language (for Zoekt)" => {
                steps.push(Step {
                    name: "Brass Section - Go".to_string(),
                    description: "Synchronizing Go environment...".to_string(),
                    command: "sudo apt-get install -y golang-go".to_string(),
                });
            }
            "Zoekt (Fast Code Search)" => {
                steps.push(Step {
                    name: "Brass Section - Ctags".to_string(),
                    description: "Installing Universal Ctags (Required for Zoekt)...".to_string(),
                    command:
                        "sudo apt-get install -y universal-ctags || sudo apt-get install -y ctags"
                            .to_string(),
                });

                steps.push(Step {
                    name: "Brass Section - Zoekt".to_string(),
                    description: "Installing Zoekt Search Engine...".to_string(),
                    command: "go install github.com/sourcegraph/zoekt/cmd/zoekt-git-index@latest && go install github.com/sourcegraph/zoekt/cmd/zoekt-indexserver@latest && go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest".to_string(),
                });
            }
            "Tmux / Tmux-RS" => {
                steps.push(Step {
                    name: "Percussion - Dependencies".to_string(),
                    description: "Installing Tmux dependencies...".to_string(),
                    command: "sudo apt-get install -y libncurses-dev libevent-dev tmux".to_string(),
                });

                steps.push(Step {
                    name: "Percussion - Tmux-RS".to_string(),
                    description: "Installing Tmux-RS from Crates.io...".to_string(),
                    command: "cargo install tmux-rs".to_string(),
                });
            }
            "Yazi (Terminal File Manager)" => {
                steps.push(Step {
                    name: "Bass Note - Yazi".to_string(),
                    description: "Ensuring Yazi is present...".to_string(),
                    command: "command -v yazi > /dev/null 2>&1 || sudo apt-get install -y yazi > /dev/null 2>&1 || cargo install --locked yazi-fm yazi-cli".to_string(),
                });
            }
            "Claude Code (by Anthropic)" => {
                steps.push(Step {
                    name: "Strings - Claude Code".to_string(),
                    description: "Installing Claude Code CLI...".to_string(),
                    command: "npm install -g @anthropic-ai/claude-code".to_string(),
                });
            }
            "Gemini CLI (by Google)" => {
                steps.push(Step {
                    name: "Strings - Gemini".to_string(),
                    description: "Installing Gemini CLI...".to_string(),
                    command: "npm install -g @google/gemini-cli".to_string(),
                });
            }
            "Codex CLI (OpenAI)" => {
                steps.push(Step {
                    name: "Strings - Codex".to_string(),
                    description: "Setting up Codex integration...".to_string(),
                    command: "echo 'Installing Codex CLI placeholder'".to_string(),
                });
            }
            "OpenCode (Independent)" => {
                steps.push(Step {
                    name: "Synthesizer - OpenCode".to_string(),
                    description: "Installing OpenCode CLI...".to_string(),
                    command: "npm install -g @opencode/cli".to_string(),
                });
            }
            "Amp (by Sourcegraph)" => {
                steps.push(Step {
                    name: "Synthesizer - Amp".to_string(),
                    description: "Installing Amp (Sourcegraph)...".to_string(),
                    command: "curl -L https://sourcegraph.com/.api/amp/v1/install.sh | sh"
                        .to_string(),
                });
            }
            _ => {}
        }
    }

    // Final Maestro Components
    steps.push(Step {
        name: "The Crescendo - Core".to_string(),
        description: "Compiling the Maestro Rust Core (Analyzers)...".to_string(),
        command: "cargo build --release".to_string(),
    });

    steps.push(Step {
        name: "The Crescendo - Frontend".to_string(),
        description: "Building Maestro Memory Dashboard...".to_string(),
        command: "cd maestro/memory/frontend && npm install && npm run build".to_string(),
    });

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
