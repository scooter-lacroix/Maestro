//! MaestroClaw doctor - system diagnostics
//!
//! Checks config validity, workspace state, CLI tool availability, and environment.
//! Provides actionable repair guidance for bootstrap completeness issues.

use crate::config::Config;
use anyhow::Result;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Ok,
    Warn,
    Error,
}

struct DiagItem {
    severity: Severity,
    category: &'static str,
    message: String,
}

impl DiagItem {
    fn ok(cat: &'static str, msg: impl Into<String>) -> Self {
        Self {
            severity: Severity::Ok,
            category: cat,
            message: msg.into(),
        }
    }
    fn warn(cat: &'static str, msg: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warn,
            category: cat,
            message: msg.into(),
        }
    }
    fn error(cat: &'static str, msg: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            category: cat,
            message: msg.into(),
        }
    }
    fn icon(&self) -> &'static str {
        match self.severity {
            Severity::Ok => "✅",
            Severity::Warn => "⚠️ ",
            Severity::Error => "❌",
        }
    }
}

/// Run full diagnostics
pub fn run(config: &Config) -> Result<()> {
    let mut items: Vec<DiagItem> = Vec::new();

    check_config(config, &mut items);
    check_workspace(config, &mut items);
    check_cli_tools(config, &mut items);
    check_environment(&mut items);
    check_repairs(config, &mut items);

    println!("🩺 MaestroClaw Doctor");
    println!();

    let mut current_cat = "";
    for item in &items {
        if item.category != current_cat {
            current_cat = item.category;
            println!("  [{current_cat}]");
        }
        println!("    {} {}", item.icon(), item.message);
    }

    let errors = items
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    let warns = items
        .iter()
        .filter(|i| i.severity == Severity::Warn)
        .count();
    let oks = items.iter().filter(|i| i.severity == Severity::Ok).count();

    println!();
    println!("  Summary: {oks} ok, {warns} warnings, {errors} errors");

    if errors > 0 {
        println!("  💡 Fix the errors above, then run `maestro claw doctor` again.");
    }

    Ok(())
}

fn check_config(config: &Config, items: &mut Vec<DiagItem>) {
    let cat = "config";
    let status = config.compute_setup_status();

    if config.config_path.exists() {
        items.push(DiagItem::ok(
            cat,
            format!("config file: {}", config.config_path.display()),
        ));
    } else {
        items.push(DiagItem::warn(
            cat,
            format!(
                "config file not found: {} (using defaults — run `maestro claw setup` to create)",
                config.config_path.display()
            ),
        ));
    }

    items.push(DiagItem::ok(
        cat,
        format!("primary tool: {}", config.primary_tool),
    ));

    // Check gateway
    if config.gateway.port > 0 {
        items.push(DiagItem::ok(
            cat,
            format!(
                "gateway bind: {}:{}",
                config.gateway.host, config.gateway.port
            ),
        ));
    } else {
        items.push(DiagItem::error(cat, "gateway port is 0 (invalid)"));
    }

    if status.secrets_configured {
        items.push(DiagItem::ok(
            cat,
            "gateway API key and webhook secret are configured and strong",
        ));
    } else if config.gateway.api_key.is_some() || config.has_webhook_secret() {
        items.push(DiagItem::warn(
            cat,
            "gateway API key and/or webhook secret need repair or rotation",
        ));
    } else {
        items.push(DiagItem::error(
            cat,
            "gateway API key and/or webhook secret missing",
        ));
    }

    if config.gateway_api_key_is_strong() {
        items.push(DiagItem::ok(cat, "gateway API key strength is acceptable"));
    } else if config.gateway.api_key.is_some() {
        items.push(DiagItem::warn(
            cat,
            "gateway API key is weak; expected `mcw_` prefix and at least 36 characters",
        ));
    }

    if config.webhook_secret_is_strong() {
        items.push(DiagItem::ok(cat, "webhook secret strength is acceptable"));
    } else if config.has_webhook_secret() {
        items.push(DiagItem::warn(
            cat,
            "webhook secret is weak; expected at least 32 characters",
        ));
    }

    if config.bootstrap.setup_timestamp.is_some() {
        items.push(DiagItem::ok(cat, "bootstrap setup timestamp recorded"));
    } else {
        items.push(DiagItem::warn(
            cat,
            "bootstrap setup timestamp missing; rerun `maestro claw setup`",
        ));
    }

    if let Some(version) = config.bootstrap.setup_version.as_deref() {
        items.push(DiagItem::ok(
            cat,
            format!("bootstrap setup version recorded: {version}"),
        ));
    } else {
        items.push(DiagItem::warn(
            cat,
            "bootstrap setup version missing; rerun `maestro claw setup`",
        ));
    }

    if status.bootstrap_state_valid {
        items.push(DiagItem::ok(
            cat,
            "bootstrap metadata matches scaffolded assets",
        ));
    } else {
        items.push(DiagItem::warn(
            cat,
            "bootstrap metadata is stale or incomplete — rerun `maestro claw setup` or restore missing assets",
        ));
    }
}

fn check_workspace(config: &Config, items: &mut Vec<DiagItem>) {
    let cat = "workspace";
    let status = config.compute_setup_status();

    if config.workspace_dir.exists() {
        items.push(DiagItem::ok(
            cat,
            format!("workspace: {}", config.workspace_dir.display()),
        ));

        // Check writability
        let probe = config
            .workspace_dir
            .join(format!(".maestroclaw_probe_{}", uuid::Uuid::new_v4()));
        match std::fs::write(&probe, b"probe") {
            Ok(()) => {
                let _ = std::fs::remove_file(&probe);
                items.push(DiagItem::ok(cat, "workspace is writable"));
            }
            Err(e) => {
                items.push(DiagItem::error(cat, format!("workspace not writable: {e}")));
            }
        }

        let cron_jobs = config.workspace_dir.join("cron").join("jobs.toml");
        if cron_jobs.exists() {
            items.push(DiagItem::ok(cat, "cron/jobs.toml present"));
        } else {
            items.push(DiagItem::warn(
                cat,
                "cron/jobs.toml missing; scaffold it for scheduled task bootstrap",
            ));
        }

        let mcp_servers = config.workspace_dir.join("mcp").join("servers.toml");
        if mcp_servers.exists() {
            items.push(DiagItem::ok(cat, "mcp/servers.toml present"));
        } else {
            items.push(DiagItem::warn(
                cat,
                "mcp/servers.toml missing; scaffold it for managed MCP bootstrap",
            ));
        }
    } else {
        items.push(DiagItem::warn(
            cat,
            format!(
                "workspace not found: {} — run `maestro claw setup`",
                config.workspace_dir.display()
            ),
        ));
    }

    if status.workspace_scaffold_complete {
        items.push(DiagItem::ok(cat, "workspace scaffold is complete"));
    } else if status.missing_scaffold_items.is_empty() {
        items.push(DiagItem::warn(
            cat,
            "workspace scaffold metadata is incomplete",
        ));
    } else {
        items.push(DiagItem::warn(
            cat,
            format!(
                "missing scaffold items: {}",
                status.missing_scaffold_items.join(", ")
            ),
        ));
    }
}

fn check_cli_tools(config: &Config, items: &mut Vec<DiagItem>) {
    let cat = "cli-tools";
    let status = config.compute_setup_status();

    let tools = [
        ("claude", &["--version"][..]),
        ("codex", &["--version"]),
        ("gemini", &["--version"]),
        ("qwen", &["--version"]),
        ("iflow", &["--version"]),
        ("amp", &["--version"]),
        ("droid", &["--version"]),
    ];

    let mut found_any = false;
    for (name, args) in &tools {
        match std::process::Command::new(name)
            .args(*args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
        {
            Ok(output) if output.status.success() => {
                let ver = String::from_utf8_lossy(&output.stdout);
                let first_line = ver.lines().next().unwrap_or("").trim();
                let display = if first_line.len() > 60 {
                    format!("{}…", &first_line[..60])
                } else {
                    first_line.to_string()
                };
                items.push(DiagItem::ok(cat, format!("{name}: {display}")));
                found_any = true;
            }
            _ => {
                items.push(DiagItem::warn(cat, format!("{name} not found in PATH")));
            }
        }
    }

    if !found_any {
        items.push(DiagItem::error(cat,
            "No CLI coding tools found! Install at least one of: claude, codex, gemini, qwen, iflow, amp, droid"
        ));
    }

    if status.primary_tool_available && status.primary_tool_in_path {
        items.push(DiagItem::ok(
            cat,
            format!(
                "primary tool `{}` is available and in PATH",
                config.primary_tool
            ),
        ));
    } else {
        items.push(DiagItem::error(
            cat,
            format!(
                "primary tool `{}` is unavailable or not in PATH",
                config.primary_tool
            ),
        ));
    }
}

fn check_environment(items: &mut Vec<DiagItem>) {
    let cat = "environment";

    // git
    check_command("git", &["--version"], cat, items);

    // Shell
    let shell = std::env::var("SHELL").unwrap_or_default();
    if shell.is_empty() {
        items.push(DiagItem::warn(cat, "$SHELL not set"));
    } else {
        items.push(DiagItem::ok(cat, format!("shell: {shell}")));
    }

    // HOME
    if std::env::var("HOME").is_ok() || std::env::var("USERPROFILE").is_ok() {
        items.push(DiagItem::ok(cat, "home directory env set"));
    } else {
        items.push(DiagItem::error(
            cat,
            "neither $HOME nor $USERPROFILE is set",
        ));
    }
}

fn check_repairs(config: &Config, items: &mut Vec<DiagItem>) {
    let cat = "repair";
    for action in config.compute_setup_status().repair_actions {
        items.push(DiagItem::warn(cat, action));
    }
}

fn check_command(cmd: &str, args: &[&str], cat: &'static str, items: &mut Vec<DiagItem>) {
    match std::process::Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
    {
        Ok(output) if output.status.success() => {
            let ver = String::from_utf8_lossy(&output.stdout);
            let line = ver.lines().next().unwrap_or("").trim();
            items.push(DiagItem::ok(cat, format!("{cmd}: {line}")));
        }
        _ => {
            items.push(DiagItem::warn(cat, format!("{cmd} not found in PATH")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diag_item_icons() {
        assert_eq!(DiagItem::ok("t", "m").icon(), "✅");
        assert_eq!(DiagItem::warn("t", "m").icon(), "⚠️ ");
        assert_eq!(DiagItem::error("t", "m").icon(), "❌");
    }
}
