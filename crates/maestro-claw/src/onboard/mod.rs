//! MaestroClaw setup/onboard wizard
//!
//! Unlike ZeroClaw which requires API keys and provider selection,
//! MaestroClaw detects locally-installed CLI coding tools and configures
//! them for direct agent loop integration.

use crate::config::{schema, AgentToolConfig, Config};
use anyhow::{Context, Result};
use std::path::Path;

const BANNER: &str = r"
    ⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡

    ███╗   ███╗ █████╗ ███████╗███████╗████████╗███████╗██████╗
    ████╗ ████║██╔══██╗██╔════╝██╔════╝╚══██╔══╝██╔════╝██╔══██╗
    ██╔████╔██║███████║█████╗  ███████╗   ██║   █████╗  ██████╔╝
    ██║╚██╔╝██║██╔══██║██╔══╝  ╚════██║   ██║   ██╔══╝  ██╔══██╗
    ██║ ╚═╝ ██║██║  ██║███████╗███████║   ██║   ███████╗██║  ██║
    ╚═╝     ╚═╝╚═╝  ╚═╝╚══════╝╚══════╝   ╚═╝   ╚══════╝╚═╝  ╚═╝
                         ██████╗██╗      █████╗ ██╗    ██╗
                        ██╔════╝██║     ██╔══██╗██║    ██║
                        ██║     ██║     ███████║██║ █╗ ██║
                        ██║     ██║     ██╔══██║██║███╗██║
                        ╚██████╗███████╗██║  ██║╚███╔███╔╝
                         ╚═════╝╚══════╝╚═╝  ╚═╝ ╚══╝╚══╝

    MaestroClaw — CLI Agent Integration for Maestro Cockpit

    ⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡
";

/// Known CLI coding tools that MaestroClaw can drive
const KNOWN_TOOLS: &[(&str, &str)] = &[
    ("claude", "Claude Code (Anthropic)"),
    ("codex", "Codex CLI (OpenAI)"),
    ("gemini", "Gemini CLI (Google)"),
    ("qwen", "Qwen Code (Alibaba)"),
    ("iflow", "iFlow Agent"),
    ("amp", "Amp CLI (Sourcegraph)"),
    ("droid", "Droid Agent"),
];

/// Detect which CLI tools are available on the system
pub fn detect_tools() -> Vec<AgentToolConfig> {
    let mut tools = Vec::new();

    for (name, _display) in KNOWN_TOOLS {
        let (available, version) = probe_tool(name);
        tools.push(AgentToolConfig {
            name: name.to_string(),
            binary_path: if available { which_path(name) } else { None },
            available,
            version,
            extra_args: Vec::new(),
        });
    }

    tools
}

pub fn probe_tool(name: &str) -> (bool, Option<String>) {
    match std::process::Command::new(name)
        .args(["--version"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
    {
        Ok(output) if output.status.success() => {
            let ver = String::from_utf8_lossy(&output.stdout);
            let line = ver.lines().next().unwrap_or("").trim().to_string();
            (true, if line.is_empty() { None } else { Some(line) })
        }
        _ => (false, None),
    }
}

fn which_path(name: &str) -> Option<String> {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        })
}

/// Generate a cryptographically strong random secret (32 bytes hex-encoded)
pub fn generate_secret() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// Ensure gateway secrets are present, generating them if needed.
/// Returns (api_key, webhook_secret, generated_api_key, generated_webhook_secret)
pub fn ensure_secrets(config: &mut Config) -> Result<(String, String, bool, bool)> {
    let mut generated_api_key = false;
    let mut generated_webhook_secret = false;

    // Generate or rotate the gateway API key if absent or weak.
    let api_key = if !config.gateway_api_key_is_strong() {
        generated_api_key = true;
        let key = format!("mcw_{}", generate_secret());
        config.gateway.api_key = Some(key.clone());
        key
    } else {
        config.gateway.api_key.as_ref().unwrap().clone()
    };

    // Generate or rotate the webhook secret if absent or weak.
    let webhook_secret = if !config.webhook_secret_is_strong() {
        generated_webhook_secret = true;
        let secret = generate_secret();
        if config.channels.webhook.is_none() {
            config.channels.webhook = Some(schema::WebhookConfig {
                secret: Some(secret.clone()),
            });
        } else {
            config.channels.webhook.as_mut().unwrap().secret = Some(secret.clone());
        }
        secret
    } else {
        config
            .channels
            .webhook
            .as_ref()
            .and_then(|w| w.secret.as_ref())
            .unwrap()
            .clone()
    };

    // Track what was generated in bootstrap metadata
    if generated_api_key {
        config.bootstrap.gateway_api_key_generated = true;
    }
    if generated_webhook_secret {
        config.bootstrap.webhook_secret_generated = true;
    }

    Ok((
        api_key,
        webhook_secret,
        generated_api_key,
        generated_webhook_secret,
    ))
}

/// Run quick setup (non-interactive)
pub fn run_quick_setup(primary_tool: Option<&str>) -> Result<Config> {
    println!("🔧 MaestroClaw Quick Setup");
    println!();

    let tools = detect_tools();
    let available_names: Vec<String> = tools
        .iter()
        .filter(|t| t.available)
        .map(|t| t.name.clone())
        .collect();

    if available_names.is_empty() {
        anyhow::bail!(
            "No CLI coding tools found! Install at least one of: {}",
            KNOWN_TOOLS
                .iter()
                .map(|(n, _)| *n)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let mut config = Config::load()?;
    let primary = if let Some(pt) = primary_tool {
        if available_names.iter().any(|n| n == pt) {
            pt.to_string()
        } else {
            anyhow::bail!(
                "Tool '{}' not found on system. Available: {}",
                pt,
                available_names.join(", ")
            );
        }
    } else if available_names.iter().any(|n| n == &config.primary_tool) {
        config.primary_tool.clone()
    } else {
        available_names[0].clone()
    };

    config.primary_tool = primary.clone();
    config.agent_tools = tools;

    // Create workspace
    std::fs::create_dir_all(&config.workspace_dir).with_context(|| {
        format!(
            "Failed to create workspace: {}",
            config.workspace_dir.display()
        )
    })?;

    // Generate secrets if needed
    let (api_key, webhook_secret, gen_api, gen_webhook) = ensure_secrets(&mut config)?;
    if gen_api {
        println!("  🔐 Generated gateway API key");
    }
    if gen_webhook {
        println!("  🔐 Generated webhook secret");
    }

    let workspace_dir = config.workspace_dir.clone();
    scaffold_workspace(&workspace_dir, &mut config)?;

    // Set up bootstrap metadata
    use std::time::{SystemTime, UNIX_EPOCH};
    config.bootstrap.setup_timestamp = Some(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
    );
    config.bootstrap.setup_version = Some(env!("CARGO_PKG_VERSION").to_string());

    // Save config
    config.save()?;

    println!("  ✅ Primary tool: {primary}");
    println!("  ✅ Available tools: {}", available_names.join(", "));
    println!("  ✅ Workspace: {}", config.workspace_dir.display());
    println!("  ✅ Config: {}", config.config_path.display());
    println!();

    // Report setup completeness
    config.setup_status = config.compute_setup_status();
    println!("  {}", config.setup_status_summary());
    println!();
    println!("  🎉 MaestroClaw is ready!");
    if gen_api {
        println!("  ⚠️  IMPORTANT: Save your gateway API key: {}", api_key);
    }
    if gen_webhook {
        println!(
            "  ⚠️  IMPORTANT: Save your webhook secret: {}",
            webhook_secret
        );
    }
    println!("     Your CLI tools are already authenticated on this machine.");

    Ok(config)
}

/// Run interactive setup wizard
pub fn run_wizard() -> Result<Config> {
    println!("{BANNER}");
    println!("  Welcome to MaestroClaw — CLI Agent Integration for Maestro Cockpit.");
    println!("  No API keys needed — MaestroClaw uses your locally-installed coding tools.");
    println!();

    // Step 1: Detect tools
    println!("  [Step 1/5] Detecting CLI coding tools...");
    let tools = detect_tools();
    let available: Vec<&AgentToolConfig> = tools.iter().filter(|t| t.available).collect();

    if available.is_empty() {
        println!("  ❌ No CLI coding tools found!");
        println!();
        println!("  Please install at least one of:");
        for (name, desc) in KNOWN_TOOLS {
            println!("    • {name} — {desc}");
        }
        anyhow::bail!("No CLI coding tools available");
    }

    println!("  Found {} tool(s):", available.len());
    for (i, tool) in available.iter().enumerate() {
        let ver = tool.version.as_deref().unwrap_or("(version unknown)");
        println!("    [{}] {} — {}", i + 1, tool.name, ver);
    }
    println!();

    let mut config = Config::load()?;

    // Step 2: Choose primary tool
    println!("  [Step 2/5] Choose primary agent tool");
    let primary = if available.len() == 1 {
        println!("  → Auto-selected: {}", available[0].name);
        available[0].name.clone()
    } else {
        let default_idx = available
            .iter()
            .position(|tool| tool.name == config.primary_tool)
            .unwrap_or(0);
        println!("  Enter tool number [{}]: ", default_idx + 1);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let idx: usize = input.trim().parse().unwrap_or(default_idx + 1);
        let idx = idx.saturating_sub(1).min(available.len() - 1);
        println!("  → Selected: {}", available[idx].name);
        available[idx].name.clone()
    };

    // Step 3: Workspace
    println!();
    println!("  [Step 3/5] Workspace setup");
    std::fs::create_dir_all(&config.workspace_dir)?;
    println!("  ✅ Workspace: {}", config.workspace_dir.display());

    // Step 4: Runtime safety
    println!();
    println!("  [Step 4/5] Runtime safety");
    println!("  ✅ Default autonomy: supervised");
    println!("  ✅ Interactive surfaces use the shared tool loop");

    // Step 5: Cron
    println!();
    println!("  [Step 5/5] Cron scheduler");
    println!("  ✅ Cron scheduler enabled by default");

    config.primary_tool = primary;
    config.agent_tools = tools;

    // Generate secrets if needed
    let (api_key, webhook_secret, gen_api, gen_webhook) = ensure_secrets(&mut config)?;
    if gen_api {
        println!("  🔐 Generated gateway API key");
    }
    if gen_webhook {
        println!("  🔐 Generated webhook secret");
    }

    let workspace_dir = config.workspace_dir.clone();
    scaffold_workspace(&workspace_dir, &mut config)?;

    // Set up bootstrap metadata
    use std::time::{SystemTime, UNIX_EPOCH};
    config.bootstrap.setup_timestamp = Some(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
    );
    config.bootstrap.setup_version = Some(env!("CARGO_PKG_VERSION").to_string());

    config.save()?;

    println!("  ✅ Config saved to: {}", config.config_path.display());
    println!();

    // Report setup completeness
    config.setup_status = config.compute_setup_status();
    println!("  {}", config.setup_status_summary());
    println!();
    println!("  🎉 MaestroClaw setup complete!");
    println!();
    println!("  Quick start:");
    println!("    maestro claw status     — Show system status");
    println!("    maestro claw doctor     — Run diagnostics");
    println!("    maestro claw daemon     — Start autonomous runtime");
    println!("    maestro claw cron list  — List scheduled tasks");
    if gen_api {
        println!();
        println!("  ⚠️  IMPORTANT: Save your gateway API key: {}", api_key);
    }
    if gen_webhook {
        println!(
            "  ⚠️  IMPORTANT: Save your webhook secret: {}",
            webhook_secret
        );
    }

    Ok(config)
}

/// Scaffold workspace files
pub fn scaffold_workspace(workspace_dir: &Path, config: &mut Config) -> Result<()> {
    std::fs::create_dir_all(workspace_dir)?;

    // Create AGENTS.md
    let agents_md = workspace_dir.join("AGENTS.md");
    if !agents_md.exists() {
        std::fs::write(
            &agents_md,
            "# MaestroClaw Agents\n\nThis file configures agent behavior.\n",
        )?;
        config.bootstrap.workspace_scaffold.agents_md = true;
    } else {
        config.bootstrap.workspace_scaffold.agents_md = true;
    }

    // Create HEARTBEAT.md
    let heartbeat_md = workspace_dir.join("HEARTBEAT.md");
    if !heartbeat_md.exists() {
        std::fs::write(
            &heartbeat_md,
            "# MaestroClaw Heartbeat\n\nThis file tracks agent health and activity.\n\n## Status\n\nLast check: N/A\n\n## Logs\n\n",
        )?;
        config.bootstrap.workspace_scaffold.heartbeat_md = true;
    } else {
        config.bootstrap.workspace_scaffold.heartbeat_md = true;
    }

    // Create skills/ directory
    let skills_dir = workspace_dir.join("skills");
    if !skills_dir.exists() {
        std::fs::create_dir_all(&skills_dir)?;
        let readme = skills_dir.join("README.md");
        std::fs::write(
            &readme,
            "# MaestroClaw Skills\n\nCustom agent skills and capabilities are defined here.\n",
        )?;
        config.bootstrap.workspace_scaffold.skills_dir = true;
    } else {
        config.bootstrap.workspace_scaffold.skills_dir = true;
    }

    // Create cost/ directory
    let cost_dir = workspace_dir.join("cost");
    if !cost_dir.exists() {
        std::fs::create_dir_all(&cost_dir)?;
        let readme = cost_dir.join("README.md");
        std::fs::write(
            &readme,
            "# Cost Tracking\n\nTrack usage and costs for different providers and agents.\n",
        )?;
    }
    config.bootstrap.workspace_scaffold.cost_dir = true;

    // Create observability/ directory
    let obs_dir = workspace_dir.join("observability");
    if !obs_dir.exists() {
        std::fs::create_dir_all(&obs_dir)?;
        let readme = obs_dir.join("README.md");
        std::fs::write(
            &readme,
            "# Observability\n\nMetrics, traces, and logs for monitoring agent behavior.\n",
        )?;
    }
    config.bootstrap.workspace_scaffold.observability_dir = true;

    // Create extensions/ directory
    let ext_dir = workspace_dir.join("extensions");
    if !ext_dir.exists() {
        std::fs::create_dir_all(&ext_dir)?;
        let readme = ext_dir.join("README.md");
        std::fs::write(
            &readme,
            "# Extensions\n\nCustom extensions and plugins for MaestroClaw.\n",
        )?;
        config.bootstrap.workspace_scaffold.extensions_dir = true;
    } else {
        config.bootstrap.workspace_scaffold.extensions_dir = true;
    }

    // Create mcp/ directory
    let mcp_dir = workspace_dir.join("mcp");
    if !mcp_dir.exists() {
        std::fs::create_dir_all(&mcp_dir)?;
        let readme = mcp_dir.join("README.md");
        std::fs::write(
            &readme,
            "# MCP Servers\n\nManaged MCP server definitions live here.\n",
        )?;
    }
    config.bootstrap.workspace_scaffold.mcp_dir = true;

    let servers_toml = mcp_dir.join("servers.toml");
    if !servers_toml.exists() {
        std::fs::write(
            &servers_toml,
            r#"# MaestroClaw managed MCP servers
#
# Example:
# [[servers]]
# name = "filesystem"
# command = "npx"
# args = ["-y", "@modelcontextprotocol/server-filesystem", "."]
# enabled = false
"#,
        )?;
    }
    config.bootstrap.workspace_scaffold.mcp_servers_file = true;

    // Create cron/ directory
    let cron_dir = workspace_dir.join("cron");
    if !cron_dir.exists() {
        std::fs::create_dir_all(&cron_dir)?;
        let readme = cron_dir.join("README.md");
        std::fs::write(
            &readme,
            "# Cron Jobs\n\nScheduled tasks and periodic jobs are configured here.\n",
        )?;
    }
    config.bootstrap.workspace_scaffold.cron_dir = true;

    let jobs_toml = cron_dir.join("jobs.toml");
    if !jobs_toml.exists() {
        std::fs::write(
            &jobs_toml,
            r#"# MaestroClaw scheduled jobs
#
# Example:
# [[jobs]]
# id = "daily-summary"
# schedule = "0 9 * * *"
# prompt = "Summarize overnight activity."
# enabled = false
"#,
        )?;
    }
    config.bootstrap.workspace_scaffold.cron_jobs_file = true;

    Ok(())
}

/// Check if all workspace assets are present
pub fn assets_present(workspace_dir: &Path) -> bool {
    workspace_dir.join("AGENTS.md").exists()
        && workspace_dir.join("HEARTBEAT.md").exists()
        && workspace_dir.join("skills").is_dir()
        && workspace_dir.join("extensions").is_dir()
        && workspace_dir.join("mcp").is_dir()
        && workspace_dir.join("mcp").join("servers.toml").exists()
        && workspace_dir.join("cost").is_dir()
        && workspace_dir.join("observability").is_dir()
        && workspace_dir.join("cron").is_dir()
        && workspace_dir.join("cron").join("jobs.toml").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn detect_tools_returns_known_list() {
        let tools = detect_tools();
        assert_eq!(tools.len(), KNOWN_TOOLS.len());
    }

    #[test]
    fn known_tools_has_expected_entries() {
        let names: Vec<&str> = KNOWN_TOOLS.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"claude"));
        assert!(names.contains(&"codex"));
        assert!(names.contains(&"gemini"));
    }

    #[test]
    fn generate_secret_produces_non_empty_string() {
        let secret = generate_secret();
        assert!(!secret.is_empty());
        assert!(secret.len() >= 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn generate_secret_is_unique() {
        let s1 = generate_secret();
        let s2 = generate_secret();
        assert_ne!(s1, s2, "Secrets should be unique");
    }

    #[test]
    fn ensure_secrets_generates_when_absent() {
        let mut config = Config::default();
        let (api_key, webhook_secret, gen_api, gen_webhook) = ensure_secrets(&mut config).unwrap();

        assert!(gen_api, "API key should be generated when absent");
        assert!(
            gen_webhook,
            "Webhook secret should be generated when absent"
        );
        assert!(api_key.starts_with("mcw_"), "API key should have prefix");
        assert!(
            !webhook_secret.is_empty(),
            "Webhook secret should not be empty"
        );
        assert!(config.bootstrap.gateway_api_key_generated);
        assert!(config.bootstrap.webhook_secret_generated);
    }

    #[test]
    fn ensure_secrets_preserves_existing() {
        let mut config = Config::default();
        config.gateway.api_key = Some("mcw_existing_key_12345678901234567890".to_string());
        config.channels.webhook = Some(schema::WebhookConfig {
            secret: Some("existing_secret_12345678901234567890".to_string()),
        });

        let (api_key, webhook_secret, gen_api, gen_webhook) = ensure_secrets(&mut config).unwrap();

        assert!(!gen_api, "API key should not be regenerated");
        assert!(!gen_webhook, "Webhook secret should not be regenerated");
        assert_eq!(api_key, "mcw_existing_key_12345678901234567890");
        assert_eq!(webhook_secret, "existing_secret_12345678901234567890");
    }

    #[test]
    fn ensure_secrets_rotates_weak_values() {
        let mut config = Config::default();
        config.gateway.api_key = Some("weak-key".to_string());
        config.channels.webhook = Some(schema::WebhookConfig {
            secret: Some("short".to_string()),
        });

        let (api_key, webhook_secret, gen_api, gen_webhook) = ensure_secrets(&mut config).unwrap();

        assert!(gen_api, "weak API key should be rotated");
        assert!(gen_webhook, "weak webhook secret should be rotated");
        assert!(api_key.starts_with("mcw_"));
        assert!(api_key.len() >= 36);
        assert!(webhook_secret.len() >= 32);
    }

    #[test]
    fn scaffold_workspace_creates_runtime_assets() {
        let temp = tempdir().unwrap();
        let mut config = Config::default();

        scaffold_workspace(temp.path(), &mut config).unwrap();

        assert!(temp.path().join("cron").join("jobs.toml").exists());
        assert!(temp.path().join("mcp").join("servers.toml").exists());
        assert!(config.bootstrap.workspace_scaffold.mcp_dir);
        assert!(config.bootstrap.workspace_scaffold.mcp_servers_file);
        assert!(config.bootstrap.workspace_scaffold.cron_jobs_file);
    }

    #[test]
    fn assets_present_requires_runtime_files() {
        let temp = tempdir().unwrap();
        let mut config = Config::default();

        scaffold_workspace(temp.path(), &mut config).unwrap();
        assert!(assets_present(temp.path()));

        std::fs::remove_file(temp.path().join("cron").join("jobs.toml")).unwrap();
        assert!(!assets_present(temp.path()));
    }
}
