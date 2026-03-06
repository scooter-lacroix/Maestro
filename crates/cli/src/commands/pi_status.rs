//! Pi-Status command for Maestro
//!
//! This module provides the `maestro pi-status` command which displays
//! the current status of Pi-Mono integration, including configuration,
//! provider authentication, and agent role assignments.

use anyhow::Result;
use maestro_pi_mono::{load_config, ModelConfig, ModelDiscovery, PiDetection};
use std::path::PathBuf;
use tracing::debug;

/// Run the pi-status command
///
/// Displays comprehensive status information about Pi-Mono integration:
/// - Configuration status (enabled/disabled, path, version)
/// - Provider authentication status
/// - Agent role assignments
pub async fn run(_config_path: Option<PathBuf>, verbose: bool, json: bool) -> Result<()> {
    debug!("Running pi-status command");

    // Load configuration - load_config returns ModelConfig (full config)
    let config = load_config()?;

    if json {
        print_status_json(&config, verbose).await?;
    } else {
        print_status_human(&config, verbose).await?;
    }

    Ok(())
}

/// Print status in human-readable format
async fn print_status_human(config: &ModelConfig, verbose: bool) -> Result<()> {
    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("  Pi-Mono Status");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    // Configuration status
    print_config_status(config);
    println!();

    // Detection status
    print_detection_status(config, verbose).await?;
    println!();

    // Provider authentication status
    print_provider_status(config, verbose).await?;
    println!();

    // Agent role assignments
    print_agent_assignments(config, verbose).await?;
    println!();

    // Summary
    print_status_summary(config);

    Ok(())
}

/// Print status in JSON format
async fn print_status_json(config: &ModelConfig, verbose: bool) -> Result<()> {
    use serde_json::json;

    let mut status = json!({
        "enabled": config.enabled,
        "path": config.path,
        "version": config.version_info,
    });

    // Add provider status
    let providers: Vec<_> = config
        .providers
        .iter()
        .map(|(name, provider)| {
            json!({
                "name": name,
                "configured": provider.is_configured,
                "env_var": provider.env_var,
            })
        })
        .collect();
    status["providers"] = json!(providers);

    // Add role assignments
    let roles: Vec<_> = config
        .role_assignments
        .iter()
        .map(|(role, assignment)| {
            json!({
                "role": role,
                "model": assignment.model_id,
                "provider": assignment.provider,
            })
        })
        .collect();
    status["roles"] = json!(roles);

    if verbose {
        status["settings"] = json!({
            "timeout": config.settings.timeout,
            "parallel_limit": config.settings.parallel_limit,
            "chain_mode": config.settings.chain_mode,
            "streaming": config.settings.streaming,
        });
    }

    println!("{}", serde_json::to_string_pretty(&status)?);
    Ok(())
}

/// Print configuration status
fn print_config_status(config: &ModelConfig) {
    println!("Configuration Status:");
    let status_symbol = if config.enabled { "✓" } else { "✗" };
    println!(
        "  Status: {} {}",
        status_symbol,
        if config.enabled {
            "Enabled"
        } else {
            "Disabled"
        }
    );
    println!(
        "  Path: {}",
        config
            .path
            .as_ref()
            .map(|p| p.as_str())
            .unwrap_or("Not configured")
    );
    println!(
        "  Version: {}",
        config
            .version_info
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("Unknown")
    );
    println!("  Schema Version: {}", config.version);
}

/// Print detection status
async fn print_detection_status(config: &ModelConfig, verbose: bool) -> Result<bool> {
    println!("Detection Status:");

    match PiDetection::detect() {
        Ok(mut detection) => {
            println!(
                "  ✓ Found pi-mono at: {}",
                detection.executable_path.display()
            );

            // Try to detect version
            match detection.detect_version().await {
                Ok(version) => {
                    println!("  ✓ Version: {}", version);
                }
                Err(e) => {
                    println!("  ⚠ Could not detect version: {}", e);
                }
            }

            // Check if detected path matches config
            if let Some(ref config_path) = config.path {
                let config_path_buf = PathBuf::from(config_path);
                if detection.executable_path == config_path_buf {
                    println!("  ✓ Matches configured path");
                } else {
                    println!("  ⚠ Detected path differs from configuration");
                    println!("    Configured: {}", config_path);
                    println!("    Detected: {}", detection.executable_path.display());
                }
            }

            if verbose {
                println!("  Capabilities:");
                println!(
                    "    Subagent: {}",
                    if detection.capabilities.subagent {
                        "✓"
                    } else {
                        "✗"
                    }
                );
                println!(
                    "    Streaming: {}",
                    if detection.capabilities.streaming {
                        "✓"
                    } else {
                        "✗"
                    }
                );
                println!(
                    "    Parallel: {}",
                    if detection.capabilities.parallel {
                        "✓"
                    } else {
                        "✗"
                    }
                );
                println!(
                    "    Chain: {}",
                    if detection.capabilities.chain {
                        "✓"
                    } else {
                        "✗"
                    }
                );
            }

            Ok(true)
        }
        Err(e) => {
            println!("  ✗ Pi-Mono not found: {}", e);
            println!("  Tip: Run 'maestro configure --pi-mono' to set up Pi-Mono");
            Ok(false)
        }
    }
}

/// Print provider authentication status
async fn print_provider_status(config: &ModelConfig, verbose: bool) -> Result<()> {
    println!("Provider Authentication:");

    if config.providers.is_empty() {
        println!("  ⚠ No providers configured");
        println!("  Tip: Run 'maestro configure --pi-mono' to set up providers");
        return Ok(());
    }

    let configured_count = config
        .providers
        .values()
        .filter(|p| p.is_configured)
        .count();
    let total_count = config.providers.len();

    println!("  Configured: {}/{}", configured_count, total_count);
    println!();

    for (_name, provider) in &config.providers {
        let status = if provider.is_configured { "✓" } else { "✗" };
        println!(
            "  {} {} ({})",
            status, provider.display_name, provider.env_var
        );
    }

    // Try model discovery to verify provider status
    if let Ok(detection) = PiDetection::detect() {
        let mut discovery = ModelDiscovery::new(detection);
        match discovery.discover_models().await {
            Ok(result) => {
                if verbose {
                    println!();
                    println!("  Discovery Results:");
                    let discovered_providers: std::collections::HashSet<_> = result
                        .models
                        .iter()
                        .map(|m| m.provider.to_lowercase())
                        .collect();

                    for (name, _provider) in &config.providers {
                        let is_discovered = discovered_providers.contains(&name.to_lowercase());
                        let status = if is_discovered { "✓" } else { "✗" };
                        println!("    {} {} - models available", status, name);
                    }
                }
            }
            Err(e) => {
                if verbose {
                    println!();
                    println!("  ⚠ Could not verify provider status: {}", e);
                }
            }
        }
    }

    Ok(())
}

/// Print agent role assignments
async fn print_agent_assignments(config: &ModelConfig, verbose: bool) -> Result<()> {
    println!("Agent Role Assignments:");

    if config.role_assignments.is_empty() {
        println!("  ⚠ No role assignments configured");
        println!("  Tip: Run 'maestro configure --pi-mono' to set up agents");
        return Ok(());
    }

    for (role_name, assignment) in &config.role_assignments {
        let reasoning_status = match assignment.use_reasoning {
            Some(true) => " [reasoning]",
            Some(false) => "",
            None => "",
        };

        println!("  {}:", role_name);
        println!(
            "    Model: {} ({}){}",
            assignment.model_id, assignment.provider, reasoning_status
        );

        if verbose {
            if let Some(fallbacks) = &assignment.fallback_models {
                if !fallbacks.is_empty() {
                    println!("    Fallbacks: {}", fallbacks.join(", "));
                }
            }
        }
    }

    Ok(())
}

/// Print status summary
fn print_status_summary(config: &ModelConfig) {
    println!("Summary:");

    let mut issues = Vec::new();

    if !config.enabled {
        issues.push("Pi-Mono is disabled");
    }

    if config.path.is_none() {
        issues.push("No path configured");
    }

    if config.providers.is_empty() {
        issues.push("No providers configured");
    } else if !config.providers.values().any(|p| p.is_configured) {
        issues.push("No providers have valid credentials");
    }

    if config.role_assignments.is_empty() {
        issues.push("No agent role assignments");
    }

    if issues.is_empty() {
        println!("  ✓ All systems operational");
    } else {
        println!("  ⚠ Issues detected:");
        for issue in &issues {
            println!("    - {}", issue);
        }
    }

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use maestro_pi_mono::config::models::{
        ExecutionSettings, PiMonoConfig, ProviderConfig, RoleAssignment,
    };
    use std::collections::HashMap;

    /// Test helper to create a test configuration
    fn create_test_config() -> PiMonoConfig {
        let mut providers = HashMap::new();
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                display_name: "Anthropic".to_string(),
                is_configured: true,
                env_var: "ANTHROPIC_API_KEY".to_string(),
            },
        );

        let mut role_assignments = HashMap::new();
        role_assignments.insert(
            "scout".to_string(),
            RoleAssignment {
                model_id: "claude-haiku-4-5".to_string(),
                provider: "anthropic".to_string(),
                fallback_models: Some(vec!["gpt-4o-mini".to_string()]),
                use_reasoning: None,
            },
        );

        PiMonoConfig {
            version: "1.0".to_string(),
            enabled: true,
            path: Some("/usr/local/bin/pi".to_string()),
            version_info: Some("0.49.3".to_string()),
            providers,
            role_assignments,
            model_preferences: vec![],
            settings: ExecutionSettings::default(),
        }
    }

    /// Test that pi-status command exists and compiles
    #[test]
    fn test_pi_status_command_exists() {
        assert!(true);
    }

    /// Test printing config status
    #[test]
    fn test_print_config_status() {
        let config = create_test_config();
        print_config_status(&config);
    }

    /// Test printing config status with minimal config
    #[test]
    fn test_print_config_status_minimal() {
        let config = PiMonoConfig::default();
        print_config_status(&config);
    }

    /// Test status summary with all good
    #[test]
    fn test_status_summary_all_good() {
        let config = create_test_config();
        print_status_summary(&config);
    }

    /// Test status summary with issues
    #[test]
    fn test_status_summary_with_issues() {
        let config = PiMonoConfig {
            enabled: false,
            ..Default::default()
        };
        print_status_summary(&config);
    }

    /// Test status JSON output
    #[tokio::test]
    async fn test_status_json_output() {
        let config = create_test_config();
        let result = print_status_json(&config, false).await;
        assert!(result.is_ok());
    }

    /// Test status JSON output with verbose
    #[tokio::test]
    async fn test_status_json_output_verbose() {
        let config = create_test_config();
        let result = print_status_json(&config, true).await;
        assert!(result.is_ok());
    }
}
