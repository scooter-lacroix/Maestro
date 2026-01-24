//! Configure command for Maestro integrations
//!
//! This module provides interactive configuration wizards for various
//! Maestro integrations, including pi-mono.

use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use maestro_pi_mono::config::wizard::ConfigWizard;
use maestro_pi_mono::config::io;
use tracing::debug;

/// Run the configure command
///
/// This is the main entry point for the configure command.
/// It delegates to specific configuration wizards based on flags.
pub async fn run(pi_mono: bool) -> Result<()> {
    if pi_mono {
        run_pi_mono_wizard().await
    } else {
        // If no specific integration is selected, show a menu
        run_configure_menu().await
    }
}

/// Run the interactive configuration menu
///
/// Shows a menu of available integrations to configure.
async fn run_configure_menu() -> Result<()> {
    let theme = ColorfulTheme::default();

    let items = vec![
        "Pi-Mono - Multi-provider AI orchestration",
        "Cancel",
    ];

    let selection = Select::with_theme(&theme)
        .with_prompt("What would you like to configure?")
        .items(&items)
        .default(0)
        .interact()?;

    match selection {
        0 => run_pi_mono_wizard().await,
        1 => {
            println!("Configuration cancelled.");
            Ok(())
        }
        _ => unreachable!(),
    }
}

/// Run the pi-mono configuration wizard
///
/// Guides users through the 5-step pi-mono configuration process:
/// 1. Detection - Verify pi-mono is installed
/// 2. Provider Review - Check provider authentication status
/// 3. Model Selection - Select models for each tier
/// 4. Role Assignment - Map models to agent roles
/// 5. Confirmation - Review and save configuration
async fn run_pi_mono_wizard() -> Result<()> {
    let theme = ColorfulTheme::default();

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("  Pi-Mono Configuration Wizard");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("This wizard will guide you through configuring Pi-Mono integration.");
    println!("Pi-Mono enables multi-provider AI orchestration for Maestro agents.");
    println!();

    // Check if user wants to proceed
    let proceed = Confirm::with_theme(&theme)
        .with_prompt("Continue with configuration?")
        .default(true)
        .interact()?;

    if !proceed {
        println!("Configuration cancelled.");
        return Ok(());
    }

    // Load existing config or create default
    let existing_config = io::load_config().unwrap_or_else(|e| {
        debug!("Could not load existing config: {}, using default", e);
        io::default_config()
    });

    let mut wizard = ConfigWizard::from_config(existing_config);

    // Step 1: Detection
    println!();
    println!("── Step 1: Pi-Mono Detection ───────────────────────────────────");
    println!("Checking for pi-mono installation...");

    match wizard.step1_detection().await {
        Ok(()) => {
            let detection = wizard.state().pi_detection.as_ref().unwrap();
            println!("  ✓ Found pi-mono at: {}", detection.executable_path.display());
            if let Some(version) = &detection.version {
                println!("  ✓ Version: {}", version);
            }
        }
        Err(e) => {
            println!("  ✗ Detection failed: {}", e);
            println!();
            println!("Pi-Mono does not appear to be installed or is not accessible.");
            println!("Please install pi-mono first:");
            println!("  https://github.com/pi-mono/pi-mono");
            return Err(anyhow::anyhow!("Pi-Mono not found: {}", e));
        }
    }

    // Step 2: Provider Review
    println!();
    println!("── Step 2: Provider Authentication ─────────────────────────────");
    println!("Checking provider API keys...");

    let configured_providers = wizard.step2_provider_review().await?;

    if configured_providers.is_empty() {
        println!("  ⚠ No providers configured with API keys.");
        println!();
        println!("To use Pi-Mono, you need to set up API keys for at least one provider.");
        println!("Supported providers and their environment variables:");

        if let Some(discovery) = wizard.state().discovery_result.as_ref() {
            for provider in &discovery.providers {
                if provider.is_configured {
                    println!("  ✓ {}: {} (configured)",
                        provider.provider, provider.env_var);
                } else {
                    println!("  ✗ {}: {} (not set)",
                        provider.provider, provider.env_var);
                }
            }
        }

        let continue_unconfigured = Confirm::with_theme(&theme)
            .with_prompt("Continue without any configured providers?")
            .default(false)
            .interact()?;

        if !continue_unconfigured {
            println!("Configuration cancelled. Please set up at least one provider API key.");
            println!("You can set environment variables in your shell profile (e.g., ~/.bashrc, ~/.zshrc)");
            return Ok(());
        }
    } else {
        println!("  ✓ Configured providers: {}", configured_providers.join(", "));
    }

    // Step 3: Model Selection
    println!();
    println!("── Step 3: Model Selection ──────────────────────────────────────");
    println!("Selecting models for each tier...");

    let tiers = wizard.state().get_tiers();
    // Clone discovery results to avoid borrow issues
    let discovery_clone = wizard.state().discovery_result.clone();

    for tier in &tiers {
        if let Some(ref discovery) = discovery_clone {
            let suggested = wizard.get_suggested_models(tier);

            if suggested.is_empty() {
                println!("  ⚠ {}: No suitable models found (skipping)", tier);
                continue;
            }

            println!();
            println!("  Tier: {}", tier);
            println!("  Suggested models:");
            for (idx, model) in suggested.iter().enumerate() {
                if let Some(model_info) = discovery.models.iter().find(|m| &m.model_id == model) {
                    println!("    {}. {} ({})", idx + 1, model, model_info.provider);
                }
            }

            // For now, auto-select the first suggested model
            // In a full interactive version, we'd prompt the user
            let selected = &suggested[0];
            if let Err(e) = wizard.step3_select_model(tier, selected) {
                debug!("Failed to select model for tier {}: {}", tier, e);
            } else {
                println!("  → Selected: {} for {}", selected, tier);
            }
        }
    }

    if wizard.state().selected_models.is_empty() {
        println!("  ⚠ No models were selected.");
        let continue_no_models = Confirm::with_theme(&theme)
            .with_prompt("Continue without model selections?")
            .default(false)
            .interact()?;

        if !continue_no_models {
            println!("Configuration cancelled.");
            return Ok(());
        }
    } else {
        println!();
        println!("  ✓ Selected {} model(s)", wizard.state().selected_models.len());
    }

    // Step 4: Role Assignment
    println!();
    println!("── Step 4: Role Assignment ──────────────────────────────────────");
    println!("Assigning models to agent roles...");

    let roles = wizard.state().get_roles();
    // Clone the default model to avoid borrow issues
    let default_model_id = wizard.state().selected_models.get("Balanced")
        .or_else(|| wizard.state().selected_models.values().next())
        .cloned();

    if wizard.state().selected_models.is_empty() {
        println!("  ⚠ No models selected, skipping role assignment.");
    } else if let Some(ref model_id) = default_model_id {
        for role in &roles {
            match wizard.step4_assign_role(role, model_id) {
                Ok(()) => {
                    println!("  → {}: {}", role, model_id);
                }
                Err(e) => {
                    debug!("Failed to assign role {}: {}", role, e);
                }
            }
        }
        println!("  ✓ Assigned {} role(s)", wizard.state().role_assignments.len());
    }

    // Step 5: Confirmation and Save
    println!();
    println!("── Step 5: Confirmation ─────────────────────────────────────────");
    println!();

    // Display configuration summary
    let config = wizard.config();
    println!("Configuration Summary:");
    println!("  Enabled: {}", config.enabled);
    println!("  Pi-Mono Path: {}", config.path.as_ref().map(|p| p.as_str()).unwrap_or("Not detected"));
    println!("  Version: {}", config.version_info.as_ref().map(|v| v.as_str()).unwrap_or("Unknown"));
    println!();

    if !config.providers.is_empty() {
        println!("  Providers:");
        for (_name, provider) in &config.providers {
            let status = if provider.is_configured { "✓" } else { "✗" };
            println!("    {} {} ({})", status, provider.display_name, provider.env_var);
        }
        println!();
    }

    if !config.model_preferences.is_empty() {
        println!("  Model Preferences:");
        for pref in &config.model_preferences {
            println!("    {} [{}] - {:?} ({})",
                pref.model_id, pref.provider, pref.tier,
                if pref.is_default { "default" } else { "optional" });
        }
        println!();
    }

    if !config.role_assignments.is_empty() {
        println!("  Role Assignments:");
        for (role, assignment) in &config.role_assignments {
            println!("    {}: {} ({})", role, assignment.model_id, assignment.provider);
        }
        println!();
    }

    let confirm = Confirm::with_theme(&theme)
        .with_prompt("Save this configuration?")
        .default(true)
        .interact()?;

    if !confirm {
        println!("Configuration not saved.");
        return Ok(());
    }

    // Save configuration
    match wizard.step5_confirm_and_save().await {
        Ok(()) => {
            let config_path = io::config_path()?;
            println!();
            println!("  ✓ Configuration saved to: {}", config_path.display());
            println!();
            println!("Pi-Mono integration is now configured!");
            println!("You can use 'maestro configure --pi-mono' to update settings later.");
            Ok(())
        }
        Err(e) => {
            println!("  ✗ Failed to save configuration: {}", e);
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_configure_command_exists() {
        // This test verifies the module compiles correctly
        // Actual functionality testing would require mocking
        assert!(true);
    }
}
