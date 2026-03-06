//! MaestroClaw OS service manager (systemd/launchd)

use crate::config::Config;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

const SERVICE_LABEL: &str = "com.maestroclaw.daemon";

pub fn handle_command(command: &str, config: &Config) -> Result<()> {
    match command {
        "install" => install(config),
        "start" => start(),
        "stop" => stop(),
        "status" => status(),
        "uninstall" => uninstall(config),
        _ => {
            println!("Usage: maestro claw service [install|start|stop|status|uninstall]");
            Ok(())
        }
    }
}

fn install(config: &Config) -> Result<()> {
    if cfg!(target_os = "macos") {
        install_macos(config)
    } else if cfg!(target_os = "linux") {
        install_linux()
    } else {
        anyhow::bail!("Service management supported on macOS and Linux only")
    }
}

fn start() -> Result<()> {
    if cfg!(target_os = "linux") {
        run_checked(&mut Command::new("systemctl").args(["--user", "daemon-reload"]))?;
        run_checked(&mut Command::new("systemctl").args([
            "--user",
            "start",
            "maestroclaw.service",
        ]))?;
        println!("✅ Service started");
    } else if cfg!(target_os = "macos") {
        let plist = macos_plist_path()?;
        run_checked(&mut Command::new("launchctl").arg("load").arg("-w").arg(&plist))?;
        run_checked(&mut Command::new("launchctl").arg("start").arg(SERVICE_LABEL))?;
        println!("✅ Service started");
    }
    Ok(())
}

fn stop() -> Result<()> {
    if cfg!(target_os = "linux") {
        let _ = run_checked(&mut Command::new("systemctl").args([
            "--user",
            "stop",
            "maestroclaw.service",
        ]));
        println!("✅ Service stopped");
    } else if cfg!(target_os = "macos") {
        let _ = run_checked(&mut Command::new("launchctl").arg("stop").arg(SERVICE_LABEL));
        println!("✅ Service stopped");
    }
    Ok(())
}

fn status() -> Result<()> {
    if cfg!(target_os = "linux") {
        let out = run_capture(&mut Command::new("systemctl").args([
            "--user",
            "is-active",
            "maestroclaw.service",
        ]))
        .unwrap_or_else(|_| "unknown".into());
        println!("Service state: {}", out.trim());
    } else if cfg!(target_os = "macos") {
        let out = run_capture(&mut Command::new("launchctl").arg("list"))?;
        let running = out.lines().any(|l| l.contains(SERVICE_LABEL));
        println!(
            "Service: {}",
            if running {
                "✅ running"
            } else {
                "❌ not loaded"
            }
        );
    }
    Ok(())
}

fn uninstall(config: &Config) -> Result<()> {
    stop()?;
    if cfg!(target_os = "linux") {
        let file = linux_service_path()?;
        if file.exists() {
            std::fs::remove_file(&file)?;
        }
        let _ = run_checked(&mut Command::new("systemctl").args(["--user", "daemon-reload"]));
        println!("✅ Service uninstalled");
    } else if cfg!(target_os = "macos") {
        let file = macos_plist_path()?;
        if file.exists() {
            std::fs::remove_file(&file)?;
        }
        println!("✅ Service uninstalled");
    }
    let _ = config;
    Ok(())
}

fn install_linux() -> Result<()> {
    let file = linux_service_path()?;
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let exe = std::env::current_exe().context("Failed to resolve executable")?;
    let unit = format!(
        "[Unit]\nDescription=MaestroClaw daemon\nAfter=network.target\n\n[Service]\nType=simple\nExecStart={} claw daemon\nRestart=always\nRestartSec=3\n\n[Install]\nWantedBy=default.target\n",
        exe.display()
    );
    std::fs::write(&file, unit)?;
    let _ = run_checked(&mut Command::new("systemctl").args(["--user", "daemon-reload"]));
    let _ = run_checked(&mut Command::new("systemctl").args([
        "--user",
        "enable",
        "maestroclaw.service",
    ]));
    println!("✅ Installed systemd user service: {}", file.display());
    Ok(())
}

fn install_macos(config: &Config) -> Result<()> {
    let file = macos_plist_path()?;
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let exe = std::env::current_exe().context("Failed to resolve executable")?;
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{SERVICE_LABEL}</string>
  <key>ProgramArguments</key><array><string>{}</string><string>claw</string><string>daemon</string></array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
</dict>
</plist>"#,
        exe.display()
    );
    std::fs::write(&file, plist)?;
    let _ = config;
    println!("✅ Installed launchd service: {}", file.display());
    Ok(())
}

fn linux_service_path() -> Result<PathBuf> {
    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    Ok(home
        .join(".config")
        .join("systemd")
        .join("user")
        .join("maestroclaw.service"))
}

fn macos_plist_path() -> Result<PathBuf> {
    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    Ok(home
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{SERVICE_LABEL}.plist")))
}

fn run_checked(cmd: &mut Command) -> Result<()> {
    let output = cmd.output().context("Failed to spawn command")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Command failed: {}", stderr.trim());
    }
    Ok(())
}

fn run_capture(cmd: &mut Command) -> Result<String> {
    let output = cmd.output().context("Failed to spawn command")?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_service_path_has_expected_suffix() {
        if let Ok(path) = linux_service_path() {
            assert!(path.to_string_lossy().ends_with("maestroclaw.service"));
        }
    }
}
