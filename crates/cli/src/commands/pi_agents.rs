//! Pi-Agents command for Maestro
//!
//! This module provides the `maestro pi-agents` command which lists
//! available agent mappings from the AgentRegistry and displays model assignments.

use anyhow::Result;
use maestro_pi_mono::{default_mappings, load_config, ModelConfig};
use std::path::PathBuf;
use tracing::debug;

/// Run the pi-agents command
///
/// Lists available agent mappings and displays model assignments.
pub async fn run(_config_path: Option<PathBuf>, verbose: bool, json: bool) -> Result<()> {
    debug!("Running pi-agents command");

    // Load configuration - load_config returns ModelConfig (full config)
    let config = load_config()?;

    if json {
        print_agents_json(&config, verbose).await?;
    } else {
        print_agents_human(&config, verbose).await?;
    }

    Ok(())
}

/// Print agents in human-readable format
async fn print_agents_human(config: &ModelConfig, verbose: bool) -> Result<()> {
    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("  Pi-Mono Agent Mappings");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    // Get default mappings
    let mappings = default_mappings();

    if mappings.is_empty() {
        println!("  No agent mappings configured");
        return Ok(());
    }

    for mapping in &mappings {
        println!("{:?} ({:?})", mapping.maestro_role, mapping.pi_agent_type);
        println!("  Description: {}", mapping.description);
        println!(
            "  Complexity: {:?} to {:?}",
            mapping.complexity_range.0, mapping.complexity_range.1
        );

        // Get model assignment
        let role_key = role_to_config_key(&mapping.maestro_role);
        if let Some(assignment) = config.role_assignments.get(&role_key) {
            println!("  Model: {} ({})", assignment.model_id, assignment.provider);
            if let Some(fallbacks) = &assignment.fallback_models {
                if !fallbacks.is_empty() {
                    println!("  Fallbacks: {}", fallbacks.join(", "));
                }
            }
        } else {
            println!("  Model: Not configured");
        }

        if verbose {
            println!("  Tool Access:");
            println!(
                "    Read: {}",
                if mapping.tool_access.can_read {
                    "✓"
                } else {
                    "✗"
                }
            );
            println!(
                "    Write: {}",
                if mapping.tool_access.can_write {
                    "✓"
                } else {
                    "✗"
                }
            );
            println!(
                "    Execute: {}",
                if mapping.tool_access.can_execute {
                    "✓"
                } else {
                    "✗"
                }
            );
            println!(
                "    Search: {}",
                if mapping.tool_access.can_search {
                    "✓"
                } else {
                    "✗"
                }
            );
            if !mapping.tool_access.allowed_tools.is_empty() {
                println!(
                    "    Allowed Tools: {}",
                    mapping
                        .tool_access
                        .allowed_tools
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }

        println!();
    }

    // Print summary
    let configured_count = config.role_assignments.len();
    let total_count = mappings.len();
    println!("Configured: {}/{} agents", configured_count, total_count);

    Ok(())
}

/// Print agents in JSON format
async fn print_agents_json(config: &ModelConfig, verbose: bool) -> Result<()> {
    use serde_json::json;

    let mappings = default_mappings();

    let agents: Vec<_> = mappings
        .iter()
        .map(|mapping| {
            let role_key = role_to_config_key(&mapping.maestro_role);
            let mut agent_json = json!({
                "role": format!("{:?}", mapping.maestro_role),
                "pi_type": format!("{:?}", mapping.pi_agent_type),
                "description": mapping.description,
                "complexity_min": format!("{:?}", mapping.complexity_range.0),
                "complexity_max": format!("{:?}", mapping.complexity_range.1),
            });

            // Add model assignment if configured
            if let Some(assignment) = config.role_assignments.get(&role_key) {
                agent_json["model"] = json!(assignment.model_id);
                agent_json["provider"] = json!(assignment.provider);
                if let Some(fallbacks) = &assignment.fallback_models {
                    agent_json["fallbacks"] = json!(fallbacks);
                }
            }

            if verbose {
                agent_json["tool_access"] = json!({
                    "can_read": mapping.tool_access.can_read,
                    "can_write": mapping.tool_access.can_write,
                    "can_execute": mapping.tool_access.can_execute,
                    "can_search": mapping.tool_access.can_search,
                    "allowed_tools": mapping.tool_access.allowed_tools.iter().cloned().collect::<Vec<_>>(),
                });
            }

            agent_json
        })
        .collect();

    let output = json!({
        "agents": agents,
        "configured": config.role_assignments.len(),
        "total": mappings.len(),
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Convert AgentRole to config key
fn role_to_config_key(role: &maestro_pi_mono::AgentRole) -> String {
    match role {
        maestro_pi_mono::AgentRole::Scout => "scout".to_string(),
        maestro_pi_mono::AgentRole::Architect => "architect".to_string(),
        maestro_pi_mono::AgentRole::Critic => "critic".to_string(),
        maestro_pi_mono::AgentRole::Kraken => "kraken".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maestro_pi_mono::{
        agents::mapping::AgentRole,
        config::models::{PiMonoConfig, RoleAssignment},
    };
    use std::collections::HashMap;

    /// Test helper to create a test configuration
    fn create_test_config() -> PiMonoConfig {
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
            role_assignments,
            ..Default::default()
        }
    }

    /// Test that pi-agents command exists and compiles
    #[test]
    fn test_pi_agents_command_exists() {
        assert!(true);
    }

    /// Test role to config key conversion
    #[test]
    fn test_role_to_config_key() {
        assert_eq!(role_to_config_key(&AgentRole::Scout), "scout");
        assert_eq!(role_to_config_key(&AgentRole::Architect), "architect");
        assert_eq!(role_to_config_key(&AgentRole::Critic), "critic");
        assert_eq!(role_to_config_key(&AgentRole::Kraken), "kraken");
    }

    /// Test print agents human format
    #[tokio::test]
    async fn test_print_agents_human() {
        let config = create_test_config();
        let result = print_agents_human(&config, false).await;
        assert!(result.is_ok());
    }

    /// Test print agents human format with verbose
    #[tokio::test]
    async fn test_print_agents_human_verbose() {
        let config = create_test_config();
        let result = print_agents_human(&config, true).await;
        assert!(result.is_ok());
    }

    /// Test print agents JSON format
    #[tokio::test]
    async fn test_print_agents_json() {
        let config = create_test_config();
        let result = print_agents_json(&config, false).await;
        assert!(result.is_ok());
    }

    /// Test print agents JSON format with verbose
    #[tokio::test]
    async fn test_print_agents_json_verbose() {
        let config = create_test_config();
        let result = print_agents_json(&config, true).await;
        assert!(result.is_ok());
    }

    /// Test with empty configuration
    #[tokio::test]
    async fn test_print_agents_empty_config() {
        let config = PiMonoConfig::default();
        let result = print_agents_human(&config, false).await;
        assert!(result.is_ok());
    }
}
