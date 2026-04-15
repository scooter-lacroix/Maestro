//! MaestroClaw doctor - system diagnostics
//!
//! Checks config validity, workspace state, CLI tool availability, and environment.
//! Provides actionable repair guidance for bootstrap completeness issues.

use crate::config::Config;
use anyhow::Result;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Ok,
    Warning,
    Error,
    Info,
}

pub struct DiagItem {
    pub severity: Severity,
    pub label: String,
    pub message: String,
}

impl DiagItem {
    fn ok(label: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            severity: Severity::Ok,
            label: label.into(),
            message: msg.into(),
        }
    }
    fn warn(label: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            label: label.into(),
            message: msg.into(),
        }
    }
    fn error(label: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            label: label.into(),
            message: msg.into(),
        }
    }
    fn info(label: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            label: label.into(),
            message: msg.into(),
        }
    }
    fn icon(&self) -> &'static str {
        match self.severity {
            Severity::Ok => "✅",
            Severity::Warning => "⚠️ ",
            Severity::Error => "❌",
            Severity::Info => "ℹ️ ",
        }
    }
}

/// Run full diagnostics
pub fn run(config: &Config) -> Result<()> {
    let items = run_diagnostics(config);

    println!("🩺 MaestroClaw Doctor");
    println!();

    for item in &items {
        println!("  [{}] {} {}", item.label, item.icon(), item.message);
    }

    let errors = items
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    let warns = items
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .count();
    let oks = items.iter().filter(|i| i.severity == Severity::Ok).count();

    println!();
    println!("  Summary: {oks} ok, {warns} warnings, {errors} errors");

    if errors > 0 {
        println!("  💡 Fix the errors above, then run `maestro claw doctor` again.");
    }

    Ok(())
}

/// Run all diagnostic checks and return results.
///
/// `tool_probe` is called to check primary-tool availability; production code
/// passes `crate::onboard::probe_tool`, tests can pass a mock.
pub(crate) fn run_diagnostics_with_probe(
    config: &Config,
    tool_probe: impl Fn(&str) -> (bool, Option<String>),
) -> Vec<DiagItem> {
    let mut items = Vec::new();

    // Primary tool check (uses injected probe for testability)
    let (tool_ok, tool_ver) = tool_probe(&config.primary_tool);
    items.push(DiagItem {
        severity: if tool_ok { Severity::Ok } else { Severity::Error },
        label: format!("Primary tool ({})", config.primary_tool),
        message: if tool_ok {
            format!("Available: {}", tool_ver.unwrap_or_default())
        } else {
            format!("'{}' not found in PATH", config.primary_tool)
        },
    });

    // Thorough checks by category
    check_config(config, &mut items);
    check_workspace(config, &mut items);
    check_cli_tools(config, &mut items);
    check_environment(&mut items);
    check_repairs(config, &mut items);

    items
}

/// Run all diagnostic checks and return results.
pub fn run_diagnostics(config: &Config) -> Vec<DiagItem> {
    run_diagnostics_with_probe(config, crate::onboard::probe_tool)
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

    // Primary tool availability is checked authoritatively via the injected
    // tool_probe in run_diagnostics_with_probe (lines 102-112), so we don't
    // duplicate that check here.
    let _ = config;
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

    /// Build a Config pointing at a temp dir so filesystem checks are deterministic.
    fn test_config(dir: &std::path::Path) -> Config {
        let mut cfg = Config::default_from(dir);
        cfg.config_path = dir.join("config.toml");
        cfg.workspace_dir = dir.join("workspace");
        cfg.primary_tool = "nonexistent_tool_xyz".into();
        cfg
    }

    /// Deterministic probe that always reports the tool as missing.
    fn not_found_probe(_name: &str) -> (bool, Option<String>) {
        (false, None)
    }

    #[test]
    fn diag_item_icons() {
        assert_eq!(DiagItem::ok("t", "m").icon(), "✅");
        assert_eq!(DiagItem::warn("t", "m").icon(), "⚠️ ");
        assert_eq!(DiagItem::error("t", "m").icon(), "❌");
        assert_eq!(DiagItem::info("t", "m").icon(), "ℹ️ ");
    }

    #[test]
    fn info_constructor_sets_info_severity() {
        let item = DiagItem::info("cat", "msg");
        assert_eq!(item.severity, Severity::Info);
        assert_eq!(item.label, "cat");
        assert_eq!(item.message, "msg");
    }

    #[test]
    fn run_diagnostics_primary_tool_missing_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = test_config(dir.path());
        let items = run_diagnostics_with_probe(&cfg, not_found_probe);

        // First item is always the primary tool probe
        assert_eq!(items[0].label, "Primary tool (nonexistent_tool_xyz)");
        assert_eq!(items[0].severity, Severity::Error);
        assert_eq!(items[0].message, "'nonexistent_tool_xyz' not found in PATH");

        // Thorough checks are now wired: we should have more than 5 items
        assert!(items.len() > 5, "expected thorough checks, got {} items", items.len());

        // All categories should be present
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.iter().any(|l| l == &"config"), "missing 'config' category");
        assert!(labels.iter().any(|l| l == &"workspace"), "missing 'workspace' category");
        assert!(labels.iter().any(|l| l == &"cli-tools"), "missing 'cli-tools' category");
        assert!(labels.iter().any(|l| l == &"environment"), "missing 'environment' category");
    }

    #[test]
    fn run_diagnostics_primary_tool_available() {
        let mock_probe = |name: &str| -> (bool, Option<String>) {
            if name == "myfakeclitool99" {
                (true, Some("myfakeclitool 9.9.9".into()))
            } else {
                (false, None)
            }
        };

        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_config(dir.path());
        cfg.primary_tool = "myfakeclitool99".into();
        let items = run_diagnostics_with_probe(&cfg, mock_probe);

        // Primary tool found
        assert_eq!(items[0].severity, Severity::Ok);
        assert_eq!(items[0].label, "Primary tool (myfakeclitool99)");
        assert_eq!(items[0].message, "Available: myfakeclitool 9.9.9");
    }

    #[test]
    fn run_diagnostics_config_checks_thorough() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = test_config(dir.path());
        let items = run_diagnostics_with_probe(&cfg, not_found_probe);

        // Config category: should warn about missing config file
        let config_items: Vec<_> = items.iter().filter(|i| i.label == "config").collect();
        assert!(!config_items.is_empty(), "expected config category items");

        // Missing config file should be a warning
        assert!(config_items.iter().any(|i| {
            i.severity == Severity::Warning
                && i.message.contains("config file not found")
        }), "expected missing config file warning");
    }

    #[test]
    fn run_diagnostics_with_existing_config_and_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("workspace")).unwrap();
        std::fs::write(dir.path().join("config.toml"), "").unwrap();
        let cfg = test_config(dir.path());
        let items = run_diagnostics_with_probe(&cfg, not_found_probe);

        // Config category: should have ok for config file existing
        let config_items: Vec<_> = items.iter().filter(|i| i.label == "config").collect();
        assert!(config_items.iter().any(|i| {
            i.severity == Severity::Ok
                && i.message.contains("config file:")
        }), "expected config file ok");

        // Workspace category: should have ok for workspace existing and writable
        let ws_items: Vec<_> = items.iter().filter(|i| i.label == "workspace").collect();
        assert!(ws_items.iter().any(|i| {
            i.severity == Severity::Ok
                && i.message.contains("workspace is writable")
        }), "expected workspace writable ok");
    }

    #[test]
    fn run_diagnostics_gateway_key_checks() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_config(dir.path());
        cfg.gateway.api_key = Some("mcw_abcdefghijklmnopqrstuvwxyz1234567890".into());
        let items = run_diagnostics_with_probe(&cfg, not_found_probe);

        // Config category: gateway key should be strong
        let config_items: Vec<_> = items.iter().filter(|i| i.label == "config").collect();
        assert!(config_items.iter().any(|i| {
            i.severity == Severity::Ok
                && i.message.contains("gateway API key strength is acceptable")
        }), "expected gateway key strength ok");

        // Also check the gateway bind line
        assert!(config_items.iter().any(|i| {
            i.severity == Severity::Ok
                && i.message.contains("gateway bind:")
        }), "expected gateway bind ok");
    }

    #[test]
    fn run_diagnostics_gateway_key_weak() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_config(dir.path());
        cfg.gateway.api_key = Some("short_key".into());
        let items = run_diagnostics_with_probe(&cfg, not_found_probe);

        // Config category: weak key should warn
        let config_items: Vec<_> = items.iter().filter(|i| i.label == "config").collect();
        assert!(config_items.iter().any(|i| {
            i.severity == Severity::Warning
                && i.message.contains("gateway API key is weak")
        }), "expected weak key warning");
    }

    #[test]
    fn run_diagnostics_workspace_cron_and_mcp_scaffold() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("workspace").join("cron")).unwrap();
        std::fs::create_dir_all(dir.path().join("workspace").join("mcp")).unwrap();
        std::fs::write(dir.path().join("workspace").join("cron").join("jobs.toml"), "").unwrap();
        std::fs::write(dir.path().join("workspace").join("mcp").join("servers.toml"), "").unwrap();
        let cfg = test_config(dir.path());
        let items = run_diagnostics_with_probe(&cfg, not_found_probe);

        let ws_items: Vec<_> = items.iter().filter(|i| i.label == "workspace").collect();
        assert!(ws_items.iter().any(|i| {
            i.severity == Severity::Ok
                && i.message.contains("cron/jobs.toml present")
        }), "expected cron/jobs.toml ok");
        assert!(ws_items.iter().any(|i| {
            i.severity == Severity::Ok
                && i.message.contains("mcp/servers.toml present")
        }), "expected mcp/servers.toml ok");
    }

    /// Smoke test for the public run_diagnostics wrapper.
    #[test]
    fn run_diagnostics_public_wrapper_smoke() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = test_config(dir.path());
        let items = run_diagnostics(&cfg);

        // First item is always the primary tool probe
        assert_eq!(
            items[0].label,
            format!("Primary tool ({})", cfg.primary_tool)
        );
        assert!(matches!(items[0].severity, Severity::Ok | Severity::Error));

        // Thorough checks should produce many items
        assert!(items.len() > 5, "expected thorough checks, got {} items", items.len());
    }
}
