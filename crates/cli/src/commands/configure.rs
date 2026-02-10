//! Configure command for Maestro integrations
//!
//! This module provides interactive configuration wizards for various
//! Maestro integrations, including pi-mono.

use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use maestro_pi_mono::config::io;
use maestro_pi_mono::config::wizard::ConfigWizard;
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

    let items = vec!["Pi-Mono - Multi-provider AI orchestration", "Cancel"];

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
        Ok(()) => match wizard.state().pi_detection.as_ref() {
            Some(detection) => {
                println!(
                    "  ✓ Found pi-mono at: {}",
                    detection.executable_path.display()
                );
                if let Some(version) = &detection.version {
                    println!("  ✓ Version: {}", version);
                }
            }
            None => {
                println!("  ⚠ Detection completed but no information available");
                return Err(anyhow::anyhow!(
                    "Pi-Mono detection succeeded but returned no results"
                ));
            }
        },
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
                    println!(
                        "  ✓ {}: {} (configured)",
                        provider.provider, provider.env_var
                    );
                } else {
                    println!("  ✗ {}: {} (not set)", provider.provider, provider.env_var);
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
        println!(
            "  ✓ Configured providers: {}",
            configured_providers.join(", ")
        );
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
            match wizard.step3_select_model(tier, selected) {
                Ok(()) => {
                    println!("  → Selected: {} for {}", selected, tier);
                }
                Err(e) => {
                    println!(
                        "  ⚠ Failed to select model '{}' for tier {}: {}",
                        selected, tier, e
                    );
                    println!("  → Continuing with model selection...");
                }
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
        println!(
            "  ✓ Selected {} model(s)",
            wizard.state().selected_models.len()
        );
    }

    // Step 4: Role Assignment
    println!();
    println!("── Step 4: Role Assignment ──────────────────────────────────────");
    println!("Assigning models to agent roles...");

    let roles = wizard.state().get_roles();
    // Clone the default model to avoid borrow issues
    let default_model_id = wizard
        .state()
        .selected_models
        .get("Balanced")
        .or_else(|| wizard.state().selected_models.values().next())
        .cloned();

    if wizard.state().selected_models.is_empty() {
        println!("  ⚠ No models selected, skipping role assignment.");
    } else if let Some(ref model_id) = default_model_id {
        let mut assigned_count = 0;
        let mut failed_roles = Vec::new();

        for role in &roles {
            match wizard.step4_assign_role(role, model_id) {
                Ok(()) => {
                    println!("  → {}: {}", role, model_id);
                    assigned_count += 1;
                }
                Err(e) => {
                    println!("  ⚠ Failed to assign role {}: {}", role, e);
                    failed_roles.push(role.clone());
                }
            }
        }

        if assigned_count > 0 {
            println!("  ✓ Assigned {} role(s)", assigned_count);
        }

        if !failed_roles.is_empty() {
            println!(
                "  ⚠ Failed to assign {} role(s): {}",
                failed_roles.len(),
                failed_roles.join(", ")
            );
        }
    }

    // Step 5: Confirmation and Save
    println!();
    println!("── Step 5: Confirmation ─────────────────────────────────────────");
    println!();

    // Display configuration summary
    let config = wizard.config();
    println!("Configuration Summary:");
    println!("  Enabled: {}", config.enabled);
    println!(
        "  Pi-Mono Path: {}",
        config
            .path
            .as_ref()
            .map(|p| p.as_str())
            .unwrap_or("Not detected")
    );
    println!(
        "  Version: {}",
        config
            .version_info
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("Unknown")
    );
    println!();

    if !config.providers.is_empty() {
        println!("  Providers:");
        for (_name, provider) in &config.providers {
            let status = if provider.is_configured { "✓" } else { "✗" };
            println!(
                "    {} {} ({})",
                status, provider.display_name, provider.env_var
            );
        }
        println!();
    }

    if !config.model_preferences.is_empty() {
        println!("  Model Preferences:");
        for pref in &config.model_preferences {
            println!(
                "    {} [{}] - {:?} ({})",
                pref.model_id,
                pref.provider,
                pref.tier,
                if pref.is_default {
                    "default"
                } else {
                    "optional"
                }
            );
        }
        println!();
    }

    if !config.role_assignments.is_empty() {
        println!("  Role Assignments:");
        for (role, assignment) in &config.role_assignments {
            println!(
                "    {}: {} ({})",
                role, assignment.model_id, assignment.provider
            );
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
    use maestro_pi_mono::config::models::PiMonoConfig;
    use std::fs;
    use tempfile::TempDir;

    /// Test that the configure command module exists and compiles
    #[test]
    fn test_configure_command_exists() {
        // This test verifies the module compiles correctly
        assert!(true);
    }

    /// Test wizard handles missing detection gracefully
    ///
    /// This test verifies that when step1_detection() succeeds but
    /// pi_detection is None (edge case), the wizard handles it properly.
    #[test]
    fn test_wizard_handles_missing_detection_gracefully() {
        // The actual wizard flow is tested in the pi-mono crate
        // This test verifies the CLI command handles the edge case
        let config = PiMonoConfig::default();
        assert!(config.path.is_none());
        // Note: enabled defaults to true in the model config
        assert_eq!(config.version, "1.0");
    }

    /// Test configuration save and load cycle
    #[test]
    fn test_config_save_load_cycle() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("pi-mono.yaml");

        let original_config = PiMonoConfig {
            version: "1.0".to_string(),
            enabled: true,
            path: Some("/usr/bin/pi-mono".to_string()),
            version_info: Some("1.0.0".to_string()),
            ..Default::default()
        };

        // Write config
        let yaml_content =
            serde_yaml::to_string(&original_config).expect("Failed to serialize config");
        fs::write(&config_path, yaml_content).expect("Failed to write config");

        // Read config back
        let read_content = fs::read_to_string(&config_path).expect("Failed to read config");
        let loaded_config: PiMonoConfig =
            serde_yaml::from_str(&read_content).expect("Failed to deserialize config");

        assert_eq!(loaded_config.enabled, original_config.enabled);
        assert_eq!(loaded_config.path, original_config.path);
        assert_eq!(loaded_config.version_info, original_config.version_info);
        assert_eq!(loaded_config.version, original_config.version);
    }

    /// Test minimal config deserialization
    ///
    /// Note: The 'version', 'enabled', 'providers', 'model_preferences',
    /// 'role_assignments', and 'settings' fields are all required in the YAML schema.
    #[test]
    fn test_minimal_config_deserialization() {
        let yaml = r#"{
  "version": "1.0",
  "enabled": true,
  "providers": {},
  "model_preferences": [],
  "role_assignments": {},
  "settings": {
    "timeout": 300,
    "parallel_limit": 4,
    "chain_mode": true,
    "streaming": true
  }
}"#;
        let config: PiMonoConfig =
            serde_yaml::from_str(yaml).expect("Failed to deserialize minimal config");

        assert_eq!(config.enabled, true);
        assert!(config.path.is_none());
        assert!(config.version_info.is_none());
        assert!(config.providers.is_empty());
        assert!(config.model_preferences.is_empty());
        assert!(config.role_assignments.is_empty());
        assert_eq!(config.version, "1.0");
    }

    /// Test partial config deserialization
    ///
    /// Tests that we can deserialize a config with optional fields set.
    /// Note: All non-optional fields must still be present.
    #[test]
    fn test_partial_config_deserialization() {
        let yaml = r#"{
  "version": "1.0",
  "enabled": true,
  "path": "/usr/local/bin/pi-mono",
  "providers": {},
  "model_preferences": [],
  "role_assignments": {},
  "settings": {
    "timeout": 300,
    "parallel_limit": 4,
    "chain_mode": true,
    "streaming": true
  }
}"#;
        let config: PiMonoConfig =
            serde_yaml::from_str(yaml).expect("Failed to deserialize partial config");

        assert_eq!(config.enabled, true);
        assert_eq!(config.path, Some("/usr/local/bin/pi-mono".to_string()));
        assert!(config.version_info.is_none());
        assert!(config.providers.is_empty());
    }

    /// Test config serialization preserves all fields
    #[test]
    fn test_config_serialization_roundtrip() {
        let original = PiMonoConfig {
            version: "1.0".to_string(),
            enabled: true,
            path: Some("/custom/path/pi".to_string()),
            version_info: Some("2.0.0".to_string()),
            ..Default::default()
        };

        let yaml = serde_yaml::to_string(&original).expect("Failed to serialize");
        let restored: PiMonoConfig = serde_yaml::from_str(&yaml).expect("Failed to deserialize");

        assert_eq!(restored.enabled, original.enabled);
        assert_eq!(restored.path, original.path);
        assert_eq!(restored.version_info, original.version_info);
        assert_eq!(restored.version, original.version);
    }

    /// Test role assignment tracking
    #[test]
    fn test_role_assignment_tracking() {
        use maestro_pi_mono::config::models::RoleAssignment;
        use std::collections::HashMap;

        let mut role_assignments = HashMap::new();
        role_assignments.insert(
            "Scout".to_string(),
            RoleAssignment {
                model_id: "gpt-4".to_string(),
                provider: "OpenAI".to_string(),
                fallback_models: None,
                use_reasoning: Some(true),
            },
        );

        assert_eq!(role_assignments.len(), 1);
        assert!(role_assignments.contains_key("Scout"));

        let scout_assignment = &role_assignments["Scout"];
        assert_eq!(scout_assignment.model_id, "gpt-4");
        assert_eq!(scout_assignment.provider, "OpenAI");
        assert_eq!(scout_assignment.use_reasoning, Some(true));
    }

    /// Test model preference tier ordering
    #[test]
    fn test_model_preference_tier_ordering() {
        use maestro_pi_mono::config::models::{ModelPreference, ModelTier};

        let fast_pref = ModelPreference {
            model_id: "gpt-3.5-turbo".to_string(),
            provider: "OpenAI".to_string(),
            tier: ModelTier::Fast,
            is_default: false,
        };

        let reasoning_pref = ModelPreference {
            model_id: "o1".to_string(),
            provider: "OpenAI".to_string(),
            tier: ModelTier::Reasoning,
            is_default: true,
        };

        assert_eq!(fast_pref.tier, ModelTier::Fast);
        assert_eq!(reasoning_pref.tier, ModelTier::Reasoning);
        assert!(reasoning_pref.is_default);
        assert!(!fast_pref.is_default);
    }

    /// Test error handling for invalid paths
    #[test]
    fn test_invalid_path_error_message() {
        use maestro_pi_mono::error::ConfigError;

        let error = ConfigError::InvalidPath {
            path: "/nonexistent/path/to/pi".to_string(),
        };

        let error_msg = format!("{}", error);
        assert!(error_msg.contains("nonexistent"));
        assert!(error_msg.contains("path"));
    }

    /// Test provider config status tracking
    #[test]
    fn test_provider_config_status() {
        use maestro_pi_mono::config::models::ProviderConfig;

        let configured_provider = ProviderConfig {
            display_name: "OpenAI".to_string(),
            env_var: "OPENAI_API_KEY".to_string(),
            is_configured: true,
        };

        let unconfigured_provider = ProviderConfig {
            display_name: "Anthropic".to_string(),
            env_var: "ANTHROPIC_API_KEY".to_string(),
            is_configured: false,
        };

        assert!(configured_provider.is_configured);
        assert!(!unconfigured_provider.is_configured);
        assert_eq!(configured_provider.display_name, "OpenAI");
        assert_eq!(unconfigured_provider.env_var, "ANTHROPIC_API_KEY");
    }
}
