use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;
use walkdir::WalkDir;

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
    pub action: StepAction,
}

pub struct Config {
    pub install_path: String,
    pub editor: String,
    pub selected_tools: Vec<String>,
}

pub enum StepAction {
    Shell(String),
    Internal(Box<dyn Fn() -> Result<Vec<String>> + Send + Sync>),
}

pub fn run_orchestra(tx: Sender<SetupEvent>, config: Config) {
    let mut steps = Vec::new();

    steps.push(Step {
        name: "The Overture".to_string(),
        description: format!("Preparing stage at {}...", config.install_path),
        action: StepAction::Shell(format!("mkdir -p {} && sleep 1", config.install_path)),
    });

    steps.push(Step {
        name: "Woodwinds".to_string(),
        description: "Installing basic utilities (curl, unzip, build-essential)...".to_string(),
        action: StepAction::Shell(
            "sudo apt-get update && sudo apt-get install -y curl unzip build-essential pkg-config libssl-dev"
                .to_string(),
        ),
    });

    if config.editor == "fresh" {
        steps.push(Step {
            name: "The Fresh Script".to_string(),
            description: "Installing FRESH (https://github.com/sinelaw/fresh)...".to_string(),
            action: StepAction::Shell(
                "curl https://raw.githubusercontent.com/sinelaw/fresh/refs/heads/master/scripts/install.sh | sh"
                    .to_string(),
            ),
        });
    }

    // Always install canonical Maestro command protocols for cross-CLI integrations.
    {
        let install_path = config.install_path.clone();
        steps.push(Step {
            name: "Maestro Protocols".to_string(),
            description: "Installing canonical Maestro command protocols...".to_string(),
            action: StepAction::Internal(Box::new(move || {
                let repo_root = find_repo_root()?;
                let maestro_home = expand_user_path(&install_path)?;
                let dst = maestro_home.join("integrations").join("commands");
                let src = repo_root.join("claude-code").join("commands");
                install_command_protocols(&src, &dst)
            })),
        });
    }

    // Handle Tooling Granularly
    for tool in &config.selected_tools {
        match tool.as_str() {
            "Go Language (for Zoekt)" => {
                steps.push(Step {
                    name: "Brass Section - Go".to_string(),
                    description: "Synchronizing Go environment...".to_string(),
                    action: StepAction::Shell("sudo apt-get install -y golang-go".to_string()),
                });
            }
            "Zoekt (Fast Code Search)" => {
                steps.push(Step {
                    name: "Brass Section - Ctags".to_string(),
                    description: "Installing Universal Ctags (Required for Zoekt)...".to_string(),
                    action: StepAction::Shell(
                        "sudo apt-get install -y universal-ctags || sudo apt-get install -y ctags"
                            .to_string(),
                    ),
                });

                steps.push(Step {
                    name: "Brass Section - Zoekt".to_string(),
                    description: "Installing Zoekt Search Engine...".to_string(),
                    action: StepAction::Shell("go install github.com/sourcegraph/zoekt/cmd/zoekt-git-index@latest && go install github.com/sourcegraph/zoekt/cmd/zoekt-indexserver@latest && go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest".to_string()),
                });
            }
            "Tmux / Tmux-RS" => {
                steps.push(Step {
                    name: "Percussion - Dependencies".to_string(),
                    description: "Installing Tmux dependencies...".to_string(),
                    action: StepAction::Shell(
                        "sudo apt-get install -y libncurses-dev libevent-dev tmux".to_string(),
                    ),
                });

                steps.push(Step {
                    name: "Percussion - Tmux-RS".to_string(),
                    description: "Installing Tmux-RS from Crates.io...".to_string(),
                    action: StepAction::Shell("cargo install tmux-rs".to_string()),
                });
            }
            "Yazi (Terminal File Manager)" => {
                steps.push(Step {
                    name: "Bass Note - Yazi".to_string(),
                    description: "Ensuring Yazi is present...".to_string(),
                    action: StepAction::Shell("command -v yazi > /dev/null 2>&1 || sudo apt-get install -y yazi > /dev/null 2>&1 || cargo install --locked yazi-fm yazi-cli".to_string()),
                });
            }
            "Claude Code (by Anthropic)" => {
                let install_path = config.install_path.clone();
                steps.push(Step {
                    name: "Strings - Claude Code".to_string(),
                    description: "Integrating Maestro into Claude Code...".to_string(),
                    action: StepAction::Internal(Box::new(move || {
                        let repo_root = find_repo_root()?;
                        let maestro_home = expand_user_path(&install_path)?;
                        let mut logs = Vec::new();

                        // Commands (copied from canonical protocols to avoid drift).
                        let src_cmd = maestro_home.join("integrations").join("commands");
                        let dst_cmd = home_dir()?.join(".claude").join("commands");
                        copy_dir_recursive(&src_cmd, &dst_cmd)?;
                        logs.push(format!(
                            "Installed Claude Code commands to {}",
                            dst_cmd.display()
                        ));

                        // Skill pack (Agent Skills standard)
                        let src_skill = repo_root.join("claude-code").join("skills").join("maestro");
                        let dst_skill = home_dir()?.join(".claude").join("skills").join("maestro");
                        copy_dir_recursive(&src_skill, &dst_skill)?;
                        logs.push(format!(
                            "Installed Claude Code skill to {}",
                            dst_skill.display()
                        ));

                        // Templates
                        let src_tpl = repo_root.join("claude-code").join("templates");
                        let dst_tpl = home_dir()?.join(".claude").join("maestro-templates");
                        copy_dir_recursive(&src_tpl, &dst_tpl)?;
                        logs.push(format!(
                            "Installed Claude Code templates to {}",
                            dst_tpl.display()
                        ));

                        // MCP config (best-effort): ensure LeIndex is reachable via `maestro mcp tool-search`.
                        let mcp_path = home_dir()?.join(".claude").join(".mcp.json");
                        let mut mcp_logs = upsert_json_server(
                            &mcp_path,
                            "mcpServers",
                            "leindex",
                            serde_json::json!({
                                "command": "maestro",
                                "args": ["mcp", "tool-search"],
                                "type": "stdio"
                            }),
                        )?;
                        logs.append(&mut mcp_logs);

                        Ok(logs)
                    })),
                });
            }
            "Gemini CLI (by Google)" => {
                let install_path = config.install_path.clone();
                steps.push(Step {
                    name: "Strings - Gemini".to_string(),
                    description: "Integrating Maestro into Gemini CLI...".to_string(),
                    action: StepAction::Internal(Box::new(move || {
                        let repo_root = find_repo_root()?;
                        let maestro_home = expand_user_path(&install_path)?;
                        let mut logs = Vec::new();

                        // Custom command pack (TOML) - install into ~/.gemini/commands/maestro/
                        let src = repo_root
                            .join("gemini-cli")
                            .join("commands")
                            .join("maestro");
                        let dst = home_dir()?.join(".gemini").join("commands").join("maestro");
                        copy_dir_recursive_with_subst(
                            &src,
                            &dst,
                            "__MAESTRO_HOME__",
                            &maestro_home.to_string_lossy(),
                        )?;
                        logs.push(format!(
                            "Installed Gemini custom commands to {}",
                            dst.display()
                        ));

                        // Skill pack (Agent Skills standard)
                        let src_skill = repo_root.join("gemini-cli").join("skills").join("maestro");
                        let dst_skill = home_dir()?.join(".gemini").join("skills").join("maestro");
                        copy_dir_recursive(&src_skill, &dst_skill)?;
                        logs.push(format!(
                            "Installed Gemini skill to {}",
                            dst_skill.display()
                        ));

                        // MCP server config in ~/.gemini/settings.json under mcpServers.leindex
                        let cfg_path = home_dir()?.join(".gemini").join("settings.json");
                        let mut cfg_logs = upsert_json_server(
                            &cfg_path,
                            "mcpServers",
                            "leindex",
                            serde_json::json!({
                                "command": "maestro",
                                "args": ["mcp", "tool-search"]
                            }),
                        )?;
                        logs.append(&mut cfg_logs);

                        Ok(logs)
                    })),
                });
            }
            "Codex CLI (OpenAI)" => {
                let install_path = config.install_path.clone();
                steps.push(Step {
                    name: "Strings - Codex".to_string(),
                    description: "Integrating Maestro into Codex CLI...".to_string(),
                    action: StepAction::Internal(Box::new(move || {
                        let repo_root = find_repo_root()?;
                        let maestro_home = expand_user_path(&install_path)?;
                        let mut logs = Vec::new();

                        // Prompts: $CODEX_HOME/prompts (default ~/.codex/prompts)
                        let codex_home = codex_home_dir();
                        let dst_prompts = codex_home.join("prompts");
                        let src_prompts = repo_root.join("codex-cli").join("prompts");
                        copy_dir_recursive_with_subst(
                            &src_prompts,
                            &dst_prompts,
                            "__MAESTRO_HOME__",
                            &maestro_home.to_string_lossy(),
                        )?;
                        logs.push(format!(
                            "Installed Codex custom prompts to {}",
                            dst_prompts.display()
                        ));

                        // MCP server config: ~/.codex/config.toml under [mcp_servers.leindex]
                        let cfg_path = codex_home.join("config.toml");
                        let mut cfg_logs = upsert_toml_mcp_server(
                            &cfg_path,
                            "leindex",
                            "maestro",
                            &["mcp", "tool-search"],
                        )?;
                        logs.append(&mut cfg_logs);

                        Ok(logs)
                    })),
                });
            }
            "OpenCode (Independent)" => {
                let install_path = config.install_path.clone();
                steps.push(Step {
                    name: "Synthesizer - OpenCode".to_string(),
                    description: "Integrating Maestro into OpenCode...".to_string(),
                    action: StepAction::Internal(Box::new(move || {
                        let repo_root = find_repo_root()?;
                        let maestro_home = expand_user_path(&install_path)?;
                        let mut logs = Vec::new();

                        // Skill pack: ~/.config/opencode/skill/maestro/
                        let src_skill = repo_root.join("opencode").join("skill").join("maestro");
                        let dst_skill = home_dir()?
                            .join(".config")
                            .join("opencode")
                            .join("skill")
                            .join("maestro");
                        copy_dir_recursive(&src_skill, &dst_skill)?;
                        logs.push(format!(
                            "Installed OpenCode skill to {}",
                            dst_skill.display()
                        ));

                        // Command files (protocols): ~/.config/opencode/commands/
                        let src_cmd = maestro_home.join("integrations").join("commands");
                        let dst_cmd = home_dir()?
                            .join(".config")
                            .join("opencode")
                            .join("commands");
                        copy_dir_recursive(&src_cmd, &dst_cmd)?;
                        logs.push(format!(
                            "Installed OpenCode command files to {}",
                            dst_cmd.display()
                        ));

                        // Update ~/.config/opencode/opencode.json:
                        // - register command templates
                        // - configure MCP server under root.mcp.leindex (OpenCode schema)
                        let cfg_path = home_dir()?
                            .join(".config")
                            .join("opencode")
                            .join("opencode.json");
                        let mut cfg_logs =
                            upsert_opencode_config(&cfg_path, &maestro_home, &dst_cmd)?;
                        logs.append(&mut cfg_logs);

                        Ok(logs)
                    })),
                });
            }
            "Qwen Code (QwenLM)" => {
                let install_path = config.install_path.clone();
                steps.push(Step {
                    name: "Strings - Qwen".to_string(),
                    description: "Integrating Maestro into Qwen Code...".to_string(),
                    action: StepAction::Internal(Box::new(move || {
                        let repo_root = find_repo_root()?;
                        let maestro_home = expand_user_path(&install_path)?;
                        let mut logs = Vec::new();

                        // Custom command pack (TOML) - install into ~/.qwen/commands/maestro/
                        let src = repo_root.join("qwen-code").join("commands").join("maestro");
                        let dst = home_dir()?.join(".qwen").join("commands").join("maestro");
                        copy_dir_recursive_with_subst(
                            &src,
                            &dst,
                            "__MAESTRO_HOME__",
                            &maestro_home.to_string_lossy(),
                        )?;
                        logs.push(format!(
                            "Installed Qwen Code custom commands to {}",
                            dst.display()
                        ));

                        // MCP server config in ~/.qwen/settings.json under mcpServers.leindex
                        let cfg_path = home_dir()?.join(".qwen").join("settings.json");
                        let mut cfg_logs = upsert_json_server(
                            &cfg_path,
                            "mcpServers",
                            "leindex",
                            serde_json::json!({
                                "command": "maestro",
                                "args": ["mcp", "tool-search"]
                            }),
                        )?;
                        logs.append(&mut cfg_logs);

                        Ok(logs)
                    })),
                });
            }
            "Amp CLI (by Sourcegraph)" => {
                steps.push(Step {
                    name: "Synthesizer - Amp".to_string(),
                    description: "Integrating Maestro into Amp CLI...".to_string(),
                    action: StepAction::Internal(Box::new(|| {
                        let mut logs = Vec::new();
                        // Skill pack (Agent Skills standard) installed to user scope
                        let repo_root = find_repo_root()?;
                        let src_skill = repo_root.join("amp-cli").join("skills").join("maestro");
                        let dst_skill = home_dir()?
                            .join(".config")
                            .join("agents")
                            .join("skills")
                            .join("maestro");
                        copy_dir_recursive(&src_skill, &dst_skill)?;
                        logs.push(format!(
                            "Installed Amp skill to {}",
                            dst_skill.display()
                        ));

                        let cfg_path = home_dir()?
                            .join(".config")
                            .join("amp")
                            .join("settings.json");
                        let mut cfg_logs = upsert_json_server(
                            &cfg_path,
                            "amp.mcpServers",
                            "leindex",
                            serde_json::json!({
                                "command": "maestro",
                                "args": ["mcp", "tool-search"]
                            }),
                        )?;
                        logs.append(&mut cfg_logs);
                        Ok(logs)
                    })),
                });
            }
            "Droid CLI (by Factory)" => {
                steps.push(Step {
                    name: "Synthesizer - Droid".to_string(),
                    description: "Integrating Maestro into Droid CLI (Factory)...".to_string(),
                    action: StepAction::Internal(Box::new(|| {
                        let mut logs = Vec::new();
                        let cfg_path = home_dir()?.join(".factory").join("mcp.json");
                        let mut cfg_logs = upsert_json_server(
                            &cfg_path,
                            "mcpServers",
                            "leindex",
                            serde_json::json!({
                                "type": "stdio",
                                "command": "maestro",
                                "args": ["mcp", "tool-search"]
                            }),
                        )?;
                        logs.append(&mut cfg_logs);
                        Ok(logs)
                    })),
                });
            }
            "pi-mono (Multi-Model CLI)" => {
                let _install_path = config.install_path.clone();
                steps.push(Step {
                    name: "Strings - pi-mono".to_string(),
                    description: "Integrating Maestro into pi-mono (Multi-Model CLI)...".to_string(),
                    action: StepAction::Internal(Box::new(move || {
                        let repo_root = find_repo_root()?;
                        let mut logs = Vec::new();

                        // Create extension directory and symlink
                        let pi_extensions = home_dir()?.join(".pi").join("extensions");
                        std::fs::create_dir_all(&pi_extensions)?;
                        logs.push(format!("Created pi-mono extensions directory: {}", pi_extensions.display()));

                        let ext_src = repo_root.join("pi-maestro");
                        let ext_dst = pi_extensions.join("pi-maestro");

                        // Remove existing symlink if present
                        if ext_dst.is_symlink() || ext_dst.exists() {
                            std::fs::remove_file(&ext_dst)?;
                            logs.push(format!("Removed existing pi-maestro extension link"));
                        }

                        // Create symlink
                        #[cfg(unix)]
                        {
                            std::os::unix::fs::symlink(&ext_src, &ext_dst)?;
                            logs.push(format!(
                                "Created symlink: {} -> {}",
                                ext_dst.display(),
                                ext_src.display()
                            ));
                        }
                        #[cfg(windows)]
                        {
                            // On Windows, use junction for directories
                            std::fs::hard_link(&ext_src, &ext_dst)?;
                            logs.push(format!(
                                "Created junction: {} -> {}",
                                ext_dst.display(),
                                ext_src.display()
                            ));
                        }

                        // Build the TypeScript extension
                        logs.push("Building pi-maestro TypeScript extension...".to_string());
                        let build_output = std::process::Command::new("npm")
                            .arg("run")
                            .arg("build")
                            .current_dir(&ext_src)
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::piped())
                            .output();

                        match build_output {
                            Ok(out) => {
                                if out.status.success() {
                                    logs.push("  TypeScript extension built successfully".to_string());
                                } else {
                                    let stderr = String::from_utf8_lossy(&out.stderr);
                                    logs.push(format!("  Build warning: {}", stderr.trim()));
                                }
                            }
                            Err(e) => {
                                logs.push(format!("  Build skipped (npm not available): {}", e));
                            }
                        }

                        // Note: npm publishing will be done separately
                        logs.push("".to_string());
                        logs.push("pi-mono extension installed locally".to_string());
                        logs.push("To install from npm (when published):".to_string());
                        logs.push("  pi install npm:@<username>/pi-maestro".to_string());
                        logs.push("".to_string());
                        logs.push("Local extension active at: ~/.pi/extensions/pi-maestro".to_string());

                        Ok(logs)
                    })),
                });
            }
            _ => {}
        }
    }

    // Final Maestro Components
    steps.push(Step {
        name: "The Crescendo - Core".to_string(),
        description: "Compiling the Maestro Rust Core (Analyzers)...".to_string(),
        action: StepAction::Shell("cargo build --release".to_string()),
    });

    steps.push(Step {
        name: "The Crescendo - Install CLI".to_string(),
        description: "Installing Maestro CLI binary to ~/.local/bin...".to_string(),
        action: StepAction::Shell(
            "mkdir -p ~/.local/bin && cp -f target/release/maestro ~/.local/bin/maestro"
                .to_string(),
        ),
    });

    // Install bundled resources (zide, layouts, etc.)
    {
        let install_path = config.install_path.clone();
        steps.push(Step {
            name: "The Crescendo - Resources".to_string(),
            description: "Installing Maestro bundled resources (zide, layouts)...".to_string(),
            action: StepAction::Internal(Box::new(move || {
                let repo_root = find_repo_root()?;
                let maestro_home = expand_user_path(&install_path)?;
                let src_resources = repo_root.join("maestro").join("leindex").join("rust").join("resources");
                let dst_resources = maestro_home.join("resources");

                let mut logs = Vec::new();

                if src_resources.exists() {
                    std::fs::create_dir_all(&dst_resources)?;
                    copy_dir_recursive(&src_resources, &dst_resources)?;
                    logs.push(format!(
                        "Installed bundled resources to {}",
                        dst_resources.display()
                    ));

                    // List what was installed
                    for entry in std::fs::read_dir(&dst_resources)?.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            logs.push(format!("  - {}", name));
                        }
                    }
                } else {
                    logs.push("  [WARN] No resources directory found in repo (development build?)".to_string());
                }

                Ok(logs)
            })),
        });
    }

    steps.push(Step {
        name: "The Crescendo - Frontend".to_string(),
        description: "Building Maestro Memory Dashboard...".to_string(),
        action: StepAction::Shell(
            "cd ../../memory/frontend && npm install && npm run build".to_string(),
        ),
    });

    let total = steps.len();
    for (i, step) in steps.into_iter().enumerate() {
        let _ = tx.send(SetupEvent::ActionStarted(step.description));
        let _ = tx.send(SetupEvent::Log(format!(
            "CONDUCTOR: Commencing {}",
            step.name
        )));

        match step.action {
            StepAction::Shell(command) => {
                // For long-running commands (like cargo build), use streaming output
                // For quick commands, use the standard approach
                let is_long_running = command.contains("cargo build")
                    || command.contains("npm install")
                    || command.contains("npm run build")
                    || command.contains("make")
                    || command.contains("apt-get");

                if is_long_running {
                    // Stream output in real-time for better user feedback
                    match Command::new("bash")
                        .arg("-c")
                        .arg(&command)
                        .stdout(std::process::Stdio::inherit())
                        .stderr(std::process::Stdio::inherit())
                        .status()
                    {
                        Ok(status) => {
                            if status.success() {
                                let _ = tx.send(SetupEvent::Log(format!("  [OK] {}", step.name)));
                                let _ = tx.send(SetupEvent::StepCompleted(i + 1, total));
                            } else {
                                let _ = tx.send(SetupEvent::Error(format!(
                                    "Step '{}' failed with exit code: {:?}",
                                    step.name, status.code()
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
                } else {
                    // Quick commands: capture output to prevent UI corruption
                    let output = Command::new("bash")
                        .arg("-c")
                        .arg(&command)
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
                                for line in s.lines().take(10) {
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
            }
            StepAction::Internal(action) => match action() {
                Ok(lines) => {
                    for line in lines {
                        let _ = tx.send(SetupEvent::Log(format!("  [OK] {}", line)));
                    }
                    let _ = tx.send(SetupEvent::StepCompleted(i + 1, total));
                }
                Err(e) => {
                    let _ = tx.send(SetupEvent::Error(format!(
                        "Step '{}' failed: {}",
                        step.name, e
                    )));
                    return;
                }
            },
        }
    }

    let _ = tx.send(SetupEvent::Finished);

    // Persist configuration using the config module
    // Convert setup Config to main Config and save
    let persistent_config = crate::config::Config {
        editor: config.editor.clone(),
        install_path: config.install_path.clone(),
        theme: crate::config::Config::default().theme,
        selected_tools: config.selected_tools.clone(),
        transparent: false,
    };
    if let Err(e) = persistent_config.save() {
        let _ = tx.send(SetupEvent::Error(format!("Failed to save config: {}", e)));
    }
}

fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().context("Failed to resolve home directory")
}

fn codex_home_dir() -> PathBuf {
    std::env::var("CODEX_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".codex")))
        .unwrap_or_else(|| PathBuf::from(".codex"))
}

fn expand_user_path(path: &str) -> Result<PathBuf> {
    if path.trim() == "~" {
        return home_dir();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return Ok(home_dir()?.join(rest));
    }
    Ok(PathBuf::from(path))
}

fn find_repo_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir().context("Failed to get current directory")?;
    for _ in 0..10 {
        if dir.join("install.sh").is_file() && dir.join("claude-code").is_dir() {
            return Ok(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    anyhow::bail!("Could not locate Maestro repo root (install.sh not found above cwd)")
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        anyhow::bail!("Source path does not exist: {}", src.display());
    }
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src)?;
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }
        if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn copy_dir_recursive_with_subst(
    src: &Path,
    dst: &Path,
    needle: &str,
    replacement: &str,
) -> Result<()> {
    if !src.exists() {
        anyhow::bail!("Source path does not exist: {}", src.display());
    }

    for entry in WalkDir::new(src) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src)?;
        let target = dst.join(rel);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }

        if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let bytes = std::fs::read(entry.path())?;
            let Ok(text) = String::from_utf8(bytes) else {
                // Binary (or non-utf8) file: copy as-is.
                std::fs::copy(entry.path(), &target)?;
                continue;
            };

            let patched = text.replace(needle, replacement);
            std::fs::write(&target, patched)?;
        }
    }

    Ok(())
}

fn install_command_protocols(src: &Path, dst: &Path) -> Result<Vec<String>> {
    std::fs::create_dir_all(dst)?;
    let mut copied = 0usize;

    for entry in std::fs::read_dir(src).with_context(|| format!("Reading {}", src.display()))? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.ends_with(".md") || name.eq_ignore_ascii_case("README.md") {
            continue;
        }
        if !name.starts_with("maestro:") {
            continue;
        }
        std::fs::copy(&path, dst.join(name))?;
        copied += 1;
    }

    Ok(vec![format!(
        "Installed {} Maestro command protocol file(s) to {}",
        copied,
        dst.display()
    )])
}

fn timestamp_suffix() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
}

fn backup_if_exists(path: &Path) -> Result<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }
    let suffix = timestamp_suffix();
    let backup = PathBuf::from(format!("{}.backup.{}", path.display(), suffix));
    std::fs::copy(path, &backup)?;
    Ok(Some(backup))
}

fn upsert_json_server(
    path: &Path,
    root_key: &str,
    server_name: &str,
    server_cfg: serde_json::Value,
) -> Result<Vec<String>> {
    let mut logs = Vec::new();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut root: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    if path.exists() {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        if !text.trim().is_empty() {
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(serde_json::Value::Object(map)) => root = map,
                Ok(_) => {
                    let backup = backup_if_exists(path)?;
                    if let Some(b) = backup {
                        logs.push(format!(
                            "Backed up non-object JSON config to {}",
                            b.display()
                        ));
                    }
                }
                Err(e) => {
                    let backup = backup_if_exists(path)?;
                    if let Some(b) = backup {
                        logs.push(format!("Backed up invalid JSON config to {}", b.display()));
                    }
                    logs.push(format!("JSON parse error (rewriting): {}", e));
                }
            }
        }
    }

    let servers_val = root
        .entry(root_key.to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let servers = ensure_json_object(servers_val);
    servers.insert(server_name.to_string(), server_cfg);
    let text = serde_json::to_string_pretty(&serde_json::Value::Object(root))?;
    std::fs::write(path, format!("{}\n", text))?;
    logs.push(format!(
        "Wrote {}.{} to {}",
        root_key,
        server_name,
        path.display()
    ));
    Ok(logs)
}

fn upsert_toml_mcp_server(
    path: &Path,
    name: &str,
    command: &str,
    args: &[&str],
) -> Result<Vec<String>> {
    let mut logs = Vec::new();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut root_table: toml::value::Table = toml::value::Table::new();
    if path.exists() {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        if !text.trim().is_empty() {
            match text.parse::<toml::Value>() {
                Ok(toml::Value::Table(t)) => root_table = t,
                Ok(_) => {
                    let backup = backup_if_exists(path)?;
                    if let Some(b) = backup {
                        logs.push(format!(
                            "Backed up non-table TOML config to {}",
                            b.display()
                        ));
                    }
                }
                Err(e) => {
                    let backup = backup_if_exists(path)?;
                    if let Some(b) = backup {
                        logs.push(format!("Backed up invalid TOML config to {}", b.display()));
                    }
                    logs.push(format!("TOML parse error (rewriting): {}", e));
                }
            }
        }
    }

    let mcp_servers_val = root_table
        .entry("mcp_servers".to_string())
        .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));
    let servers_table = ensure_toml_table(mcp_servers_val);
    servers_table.insert(
        name.to_string(),
        toml::Value::Table(toml_server_table(command, args)),
    );
    let text = toml::to_string_pretty(&toml::Value::Table(root_table))?;
    std::fs::write(path, text)?;
    logs.push(format!(
        "Wrote [mcp_servers.{}] to {}",
        name,
        path.display()
    ));
    Ok(logs)
}

fn toml_server_table(command: &str, args: &[&str]) -> toml::value::Table {
    let mut t = toml::value::Table::new();
    t.insert(
        "command".to_string(),
        toml::Value::String(command.to_string()),
    );
    t.insert(
        "args".to_string(),
        toml::Value::Array(
            args.iter()
                .map(|s| toml::Value::String((*s).to_string()))
                .collect(),
        ),
    );
    t
}

fn upsert_opencode_config(
    path: &Path,
    maestro_home: &Path,
    opencode_commands_dir: &Path,
) -> Result<Vec<String>> {
    let mut logs = Vec::new();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut root: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    if path.exists() {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        if !text.trim().is_empty() {
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(serde_json::Value::Object(map)) => root = map,
                Ok(_) => {
                    let backup = backup_if_exists(path)?;
                    if let Some(b) = backup {
                        logs.push(format!(
                            "Backed up non-object OpenCode config to {}",
                            b.display()
                        ));
                    }
                }
                Err(e) => {
                    let backup = backup_if_exists(path)?;
                    if let Some(b) = backup {
                        logs.push(format!(
                            "Backed up invalid OpenCode config to {}",
                            b.display()
                        ));
                    }
                    logs.push(format!("OpenCode JSON parse error (rewriting): {}", e));
                }
            }
        }
    }

    // Command registry
    let cmd_val = root
        .entry("command".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let cmd_map = ensure_json_object(cmd_val);
    upsert_opencode_commands(cmd_map, maestro_home, opencode_commands_dir)?;
    upsert_opencode_mcp(&mut root)?;

    let text = serde_json::to_string_pretty(&serde_json::Value::Object(root))?;
    std::fs::write(path, format!("{}\n", text))?;
    logs.push(format!("Wrote OpenCode config to {}", path.display()));
    Ok(logs)
}

fn upsert_opencode_commands(
    cmd_map: &mut serde_json::Map<String, serde_json::Value>,
    maestro_home: &Path,
    opencode_commands_dir: &Path,
) -> Result<()> {
    let _ = maestro_home;
    let commands_dir = opencode_commands_dir;
    let mk = |template: String, description: &str| {
        serde_json::json!({
            "template": template,
            "description": description
        })
    };

    cmd_map.insert(
        "maestro".to_string(),
        mk(
            "Load Maestro. Usage: /maestro setup | newTrack | implement | status | revert | configure. Args: $ARGUMENTS".to_string(),
            "Maestro spec-driven development framework",
        ),
    );

    let file_template = |cmd: &str| {
        format!(
            "Read and execute from {} with args: $ARGUMENTS",
            commands_dir.join(cmd).display()
        )
    };

    cmd_map.insert(
        "maestro:setup".to_string(),
        mk(file_template("maestro:setup.md"), "Maestro setup command"),
    );
    cmd_map.insert(
        "maestro:newTrack".to_string(),
        mk(
            file_template("maestro:newTrack.md"),
            "Maestro newTrack command",
        ),
    );
    cmd_map.insert(
        "maestro:implement".to_string(),
        mk(
            file_template("maestro:implement.md"),
            "Maestro implement command",
        ),
    );
    cmd_map.insert(
        "maestro:status".to_string(),
        mk(file_template("maestro:status.md"), "Maestro status command"),
    );
    cmd_map.insert(
        "maestro:revert".to_string(),
        mk(file_template("maestro:revert.md"), "Maestro revert command"),
    );
    cmd_map.insert(
        "maestro:configure".to_string(),
        mk(
            file_template("maestro:configure.md"),
            "Maestro configure command",
        ),
    );
    cmd_map.insert(
        "maestro:orchestrate".to_string(),
        mk(
            file_template("maestro:orchestrate.md"),
            "Maestro orchestrate command",
        ),
    );
    cmd_map.insert(
        "maestro:memory".to_string(),
        mk(file_template("maestro:memory.md"), "Maestro memory command"),
    );
    cmd_map.insert(
        "maestro:tui".to_string(),
        mk(file_template("maestro:tui.md"), "Maestro TUI command"),
    );
    cmd_map.insert(
        "maestro:leindex".to_string(),
        mk(
            file_template("maestro:leindex.md"),
            "Maestro LeIndex command",
        ),
    );

    Ok(())
}

fn upsert_opencode_mcp(root: &mut serde_json::Map<String, serde_json::Value>) -> Result<()> {
    let mcp_val = root
        .entry("mcp".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let mcp_map = ensure_json_object(mcp_val);
    mcp_map.insert(
        "leindex".to_string(),
        serde_json::json!({
            "command": ["maestro", "mcp", "tool-search"],
            "environment": {}
        }),
    );
    Ok(())
}

fn ensure_json_object(
    value: &mut serde_json::Value,
) -> &mut serde_json::Map<String, serde_json::Value> {
    if !value.is_object() {
        *value = serde_json::Value::Object(serde_json::Map::new());
    }
    value
        .as_object_mut()
        .expect("value is object after normalization")
}

fn ensure_toml_table(value: &mut toml::Value) -> &mut toml::value::Table {
    if !value.is_table() {
        *value = toml::Value::Table(toml::value::Table::new());
    }
    value
        .as_table_mut()
        .expect("value is table after normalization")
}

/// Save setup configuration to the config file
/// This writes the selected tools and paths to the config module's config file
fn save_setup_config(config: &Config) -> Result<()> {
    use std::fs;
    use std::io::Write;

    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?
        .join("maestro");

    fs::create_dir_all(&config_dir)
        .context("Failed to create config directory")?;

    let config_path = config_dir.join("config.toml");

    // Build TOML config
    let mut toml_content = String::new();
    toml_content.push_str("# Maestro Configuration\n");
    toml_content.push_str(&format!("editor = \"{}\"\n", config.editor));
    toml_content.push_str(&format!("install_path = \"{}\"\n", config.install_path));

    if !config.selected_tools.is_empty() {
        toml_content.push_str("selected_tools = [\n");
        for tool in &config.selected_tools {
            toml_content.push_str(&format!("    \"{}\",\n", tool));
        }
        toml_content.push_str("]\n");
    }

    let mut file = fs::File::create(&config_path)
        .context("Failed to create config file")?;
    file.write_all(toml_content.as_bytes())
        .context("Failed to write config file")?;

    Ok(())
}
