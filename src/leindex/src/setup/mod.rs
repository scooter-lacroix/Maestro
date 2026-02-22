use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use walkdir::WalkDir;

pub mod distro;
pub mod package_manager;
pub mod password;

pub use distro::{detect_distro, Distro};
pub use package_manager::{
    get_build_tools_install_command, get_package_manager, get_package_name, get_package_names,
    PackageManager, PackagePurpose,
};
pub use password::PasswordCache;

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
    pub password_cache: Arc<PasswordCache>,
    pub distro: Distro,
}

pub enum StepAction {
    Shell(String),
    Internal(Box<dyn Fn() -> Result<Vec<String>> + Send + Sync>),
}

pub fn run_orchestra(tx: Sender<SetupEvent>, config: Config) {
    let mut steps = Vec::new();
    // Use the distribution passed in config
    let distro = config.distro;
    let pm = get_package_manager(distro);
    let pm_name = pm.name().to_string();

    steps.push(Step {
        name: "The Overture".to_string(),
        description: format!("Preparing stage at {}...", config.install_path),
        action: StepAction::Shell(format!("mkdir -p {} && sleep 1", config.install_path)),
    });

    // Woodwinds: Basic utilities using distribution-appropriate package manager
    let basic_packages = get_package_names(
        &[
            PackagePurpose::Curl,
            PackagePurpose::Unzip,
            PackagePurpose::PkgConfig,
            PackagePurpose::OpenSSL,
        ],
        distro,
    );

    let woodwinds_cmd = match distro {
        Distro::Debian => {
            let pkgs = basic_packages.join(" ");
            format!(
                "sudo apt-get update && sudo apt-get install -y build-essential {}",
                pkgs
            )
        }
        Distro::Arch => {
            let pkgs = basic_packages.join(" ");
            format!("sudo pacman -S --noconfirm --needed base-devel {}", pkgs)
        }
        Distro::Fedora => {
            let pkgs = basic_packages.join(" ");
            format!("sudo dnf install -y @development-tools {}", pkgs)
        }
        _ => format!(
            "# Please install basic build tools: {}",
            basic_packages.join(" ")
        ),
    };

    steps.push(Step {
        name: "Woodwinds".to_string(),
        description: format!("[{}] Installing basic utilities...", pm_name),
        action: StepAction::Shell(woodwinds_cmd),
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
                let go_pkg = get_package_name(PackagePurpose::Go, distro).unwrap_or("golang-go");
                let go_cmd = pm.install_command(&[go_pkg]);
                steps.push(Step {
                    name: "Brass Section - Go".to_string(),
                    description: format!("[{}] Synchronizing Go environment...", pm_name),
                    action: StepAction::Shell(go_cmd),
                });
            }
            "Zoekt (Fast Code Search)" => {
                // Install ctags first
                let ctags_pkg = get_package_name(PackagePurpose::Ctags, distro).unwrap_or("ctags");
                let ctags_cmd = match distro {
                    Distro::Debian => format!(
                        "sudo apt-get install -y {} || sudo apt-get install -y ctags",
                        ctags_pkg
                    ),
                    _ => pm.install_command(&[ctags_pkg]),
                };
                steps.push(Step {
                    name: "Brass Section - Ctags".to_string(),
                    description: format!("[{}] Installing Universal Ctags...", pm_name),
                    action: StepAction::Shell(ctags_cmd),
                });

                steps.push(Step {
                    name: "Brass Section - Zoekt".to_string(),
                    description: "Installing Zoekt Search Engine...".to_string(),
                    action: StepAction::Shell("go install github.com/sourcegraph/zoekt/cmd/zoekt-git-index@latest && go install github.com/sourcegraph/zoekt/cmd/zoekt-indexserver@latest && go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest".to_string()),
                });
            }
            "Tmux / Tmux-RS" => {
                let tmux_pkgs = get_package_names(
                    &[
                        PackagePurpose::Ncurses,
                        PackagePurpose::LibEvent,
                        PackagePurpose::Tmux,
                    ],
                    distro,
                );
                let tmux_cmd = match distro {
                    Distro::Debian => format!("sudo apt-get install -y {}", tmux_pkgs.join(" ")),
                    Distro::Arch => format!(
                        "sudo pacman -S --noconfirm --needed {}",
                        tmux_pkgs.join(" ")
                    ),
                    Distro::Fedora => format!("sudo dnf install -y {}", tmux_pkgs.join(" ")),
                    _ => format!(
                        "# Please install tmux dependencies: {}",
                        tmux_pkgs.join(" ")
                    ),
                };
                steps.push(Step {
                    name: "Percussion - Dependencies".to_string(),
                    description: format!("[{}] Installing Tmux dependencies...", pm_name),
                    action: StepAction::Shell(tmux_cmd),
                });

                steps.push(Step {
                    name: "Percussion - Tmux-RS".to_string(),
                    description: "Installing Tmux-RS from Crates.io...".to_string(),
                    action: StepAction::Shell("cargo install tmux-rs".to_string()),
                });
            }
            "Yazi (Terminal File Manager)" => {
                let yazi_pkg = get_package_name(PackagePurpose::Yazi, distro).unwrap_or("yazi");
                let yazi_cmd = match distro {
                    Distro::Debian => format!("command -v yazi > /dev/null 2>&1 || sudo apt-get install -y {} > /dev/null 2>&1 || cargo install --locked yazi-fm yazi-cli", yazi_pkg),
                    Distro::Arch => format!("command -v yazi > /dev/null 2>&1 || sudo pacman -S --noconfirm --needed {} || cargo install --locked yazi-fm yazi-cli", yazi_pkg),
                    Distro::Fedora => format!("command -v yazi > /dev/null 2>&1 || sudo dnf install -y {} || cargo install --locked yazi-fm yazi-cli", yazi_pkg),
                    _ => "command -v yazi > /dev/null 2>&1 || cargo install --locked yazi-fm yazi-cli".to_string(),
                };
                steps.push(Step {
                    name: "Bass Note - Yazi".to_string(),
                    description: format!("[{}] Ensuring Yazi is present...", pm_name),
                    action: StepAction::Shell(yazi_cmd),
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
                        let src_skill =
                            repo_root.join("claude-code").join("skills").join("maestro");
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
                        logs.push(format!("Installed Gemini skill to {}", dst_skill.display()));

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
                        logs.push(format!("Installed Amp skill to {}", dst_skill.display()));

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
                    description: "Integrating Maestro into pi-mono (Multi-Model CLI)..."
                        .to_string(),
                    action: StepAction::Internal(Box::new(move || {
                        let repo_root = find_repo_root()?;
                        let mut logs = Vec::new();

                        // Create extension directory and symlink
                        let pi_extensions = home_dir()?.join(".pi").join("extensions");
                        std::fs::create_dir_all(&pi_extensions)?;
                        logs.push(format!(
                            "Created pi-mono extensions directory: {}",
                            pi_extensions.display()
                        ));

                        let ext_src = repo_root.join("pi-maestro");
                        let ext_dst = pi_extensions.join("pi-maestro");

                        // Remove existing symlink if present
                        if ext_dst.is_symlink() || ext_dst.exists() {
                            std::fs::remove_file(&ext_dst)?;
                            logs.push("Removed existing pi-maestro extension link".to_string());
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
                                    logs.push(
                                        "  TypeScript extension built successfully".to_string(),
                                    );
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
                        logs.push(
                            "Local extension active at: ~/.pi/extensions/pi-maestro".to_string(),
                        );

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
        description: "Compiling canonical Maestro CLI (crates/cli)...".to_string(),
        action: StepAction::Internal(Box::new(|| {
            let repo_root = find_repo_root()?;
            let manifest_path = repo_root.join("crates").join("cli").join("Cargo.toml");

            let output = std::process::Command::new("cargo")
                .arg("build")
                .arg("--release")
                .arg("--manifest-path")
                .arg(&manifest_path)
                .arg("--bin")
                .arg("maestro")
                .current_dir(&repo_root)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
                .context("Failed to launch cargo build for crates/cli")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!(
                    "Failed to build canonical Maestro CLI (crates/cli): {}",
                    stderr.trim()
                );
            }

            Ok(vec![
                format!(
                    "Built canonical Maestro CLI via {}",
                    manifest_path.display()
                ),
                "Verified build target: crates/cli (not leindex-core shim)".to_string(),
            ])
        })),
    });

    steps.push(Step {
        name: "The Crescendo - Install CLI".to_string(),
        description: "Installing Maestro CLI binary to ~/.local/bin...".to_string(),
        action: StepAction::Internal(Box::new(|| {
            let repo_root = find_repo_root()?;
            let src_bin = repo_root.join("target").join("release").join("maestro");
            if !src_bin.exists() {
                anyhow::bail!(
                    "Expected canonical binary at {} but it does not exist",
                    src_bin.display()
                );
            }

            let dst_dir = home_dir()?.join(".local").join("bin");
            std::fs::create_dir_all(&dst_dir)?;
            let dst_bin = dst_dir.join("maestro");
            std::fs::copy(&src_bin, &dst_bin).with_context(|| {
                format!(
                    "Failed copying Maestro CLI from {} to {}",
                    src_bin.display(),
                    dst_bin.display()
                )
            })?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&dst_bin)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&dst_bin, perms)?;
            }

            let help_output = std::process::Command::new(&dst_bin)
                .arg("--help")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
                .with_context(|| format!("Failed to execute {} --help", dst_bin.display()))?;

            if !help_output.status.success() {
                let stderr = String::from_utf8_lossy(&help_output.stderr);
                anyhow::bail!("Installed maestro --help failed: {}", stderr.trim());
            }

            let help_text = String::from_utf8_lossy(&help_output.stdout);
            for required in ["orchestrate", "pi-status", "pi-test", "pi-agents"] {
                if !help_text.contains(required) {
                    anyhow::bail!(
                        "Installed binary missing '{}' command; non-canonical binary likely installed",
                        required
                    );
                }
            }

            Ok(vec![
                format!("Installed Maestro CLI to {}", dst_bin.display()),
                "Verified command surface includes orchestrate/pi-* commands".to_string(),
            ])
        })),
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
                let src_resources = repo_root
                    .join("maestro")
                    .join("leindex")
                    .join("rust")
                    .join("resources");
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
                    logs.push(
                        "  [WARN] No resources directory found in repo (development build?)"
                            .to_string(),
                    );
                }

                Ok(logs)
            })),
        });
    }

    // Build frontend using internal action for correct path resolution
    steps.push(Step {
        name: "The Crescendo - Frontend".to_string(),
        description: "Building Maestro Memory Dashboard...".to_string(),
        action: StepAction::Internal(Box::new(|| {
            let repo_root = find_repo_root()?;
            let frontend_path = repo_root.join("maestro").join("memory").join("frontend");
            let mut logs = Vec::new();

            if !frontend_path.exists() {
                logs.push("Frontend directory not found, skipping build".to_string());
                return Ok(logs);
            }

            logs.push(format!("Building frontend at {}", frontend_path.display()));

            // Run npm install
            let npm_install = std::process::Command::new("npm")
                .arg("install")
                .current_dir(&frontend_path)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output();

            match npm_install {
                Ok(out) => {
                    if !out.stdout.is_empty() {
                        for line in String::from_utf8_lossy(&out.stdout).lines().take(3) {
                            logs.push(format!("  npm: {}", line));
                        }
                    }
                    if !out.status.success() {
                        logs.push(format!(
                            "npm install failed: {}",
                            String::from_utf8_lossy(&out.stderr)
                        ));
                        return Ok(logs);
                    }
                }
                Err(e) => {
                    logs.push(format!("npm install error: {}", e));
                    return Ok(logs);
                }
            }

            // Run npm run build
            let npm_build = std::process::Command::new("npm")
                .arg("run")
                .arg("build")
                .current_dir(&frontend_path)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output();

            match npm_build {
                Ok(out) => {
                    if !out.stdout.is_empty() {
                        for line in String::from_utf8_lossy(&out.stdout).lines().take(3) {
                            logs.push(format!("  build: {}", line));
                        }
                    }
                    if !out.status.success() {
                        logs.push(format!(
                            "npm build failed: {}",
                            String::from_utf8_lossy(&out.stderr)
                        ));
                    } else {
                        logs.push("Frontend built successfully".to_string());
                    }
                }
                Err(e) => {
                    logs.push(format!("npm build error: {}", e));
                }
            }

            Ok(logs)
        })),
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
                // ALL commands: capture output to prevent UI corruption
                // Never use Stdio::inherit() - it corrupts the TUI
                let needs_sudo = command.contains("sudo");
                let clean_command = if needs_sudo {
                    command.replace("sudo ", "")
                } else {
                    command.clone()
                };
                let is_long_running = command.contains("cargo build")
                    || command.contains("npm install")
                    || command.contains("npm run build")
                    || command.contains("make");

                // For sudo commands, we need to handle them specially
                let output = if needs_sudo {
                    // Check if we have a cached password
                    if !config.password_cache.is_valid() {
                        let _ = tx.send(SetupEvent::Log("[sudo] password required".to_string()));
                        // Wait for password to be provided via TUI
                        while !config.password_cache.is_valid() {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                    }
                    config
                        .password_cache
                        .sudo_with_password(&clean_command)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                } else {
                    Command::new("bash")
                        .arg("-c")
                        .arg(&command)
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .stdin(std::process::Stdio::null())
                        .output()
                };

                match output {
                    Ok(out) => {
                        // Send stdout to TUI logs
                        if !out.stdout.is_empty() {
                            let s = String::from_utf8_lossy(&out.stdout);
                            let max_lines = if is_long_running { 10 } else { 5 };
                            for line in s.lines().take(max_lines) {
                                let _ = tx.send(SetupEvent::Log(format!("  [OUT] {}", line)));
                            }
                        }
                        // Send stderr to TUI logs (but filter password prompts)
                        if !out.stderr.is_empty() {
                            let s = String::from_utf8_lossy(&out.stderr);
                            let max_lines = if is_long_running { 10 } else { 10 };
                            for line in s.lines().take(max_lines) {
                                // Don't send password prompts to logs - they'll be handled separately
                                if !line.contains("[sudo]") || !line.contains("password") {
                                    let prefix = if is_long_running {
                                        "  [WARN] "
                                    } else {
                                        "  [ERR] "
                                    };
                                    let _ = tx.send(SetupEvent::Log(format!("{}{}", prefix, line)));
                                }
                            }
                        }

                        if out.status.success() {
                            let _ = tx.send(SetupEvent::Log(format!("  [OK] {}", step.name)));
                            let _ = tx.send(SetupEvent::StepCompleted(i + 1, total));
                        } else {
                            // Check if it's a password error
                            let stderr = String::from_utf8_lossy(&out.stderr);
                            if stderr.contains("Sorry") || stderr.contains("incorrect") {
                                let _ = tx.send(SetupEvent::Error(
                                    "Password authentication failed. Please check your password and try again.".to_string()
                                ));
                            } else {
                                let _ = tx.send(SetupEvent::Error(format!(
                                    "Step '{}' failed with exit code: {:?}",
                                    step.name,
                                    out.status.code()
                                )));
                            }
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
