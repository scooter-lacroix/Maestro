//! Implement command wrapper with Pi-Mono support
//!
//! This module wraps the leindex-core implement command and adds
//! support for Pi-Mono subagent execution modes.

use anyhow::Result;
use leindex_core::cli::implement::ImplementSessionTarget;

/// Re-export the original implement module for fallback
pub use leindex_core::cli::implement as leindex_implement;

use maestro_pi_mono::{agents::mapping::PiAgentType, load_config, PiDetection, SubagentRunner};
use std::path::PathBuf;
use tracing::{debug, info};

/// Run the implement command with optional Pi-Mono support
///
/// If any pi-mono flags are provided, executes using SubagentRunner.
/// Otherwise, falls through to the standard leindex-core implement command.
pub async fn run(
    command: String,
    description: Vec<String>,
    session: ImplementSessionTarget,
    tool: String,
    path: Option<PathBuf>,
    title: Option<String>,
    pi_agent: Option<String>,
    pi_chain: Option<Vec<String>>,
    pi_parallel: Option<Vec<String>>,
) -> Result<()> {
    // Check if any pi-mono mode is requested
    let pi_mode = PiMode::from_flags(pi_agent, pi_chain, pi_parallel);

    if let Some(mode) = pi_mode {
        debug!("Pi-Mono mode requested: {:?}", mode);
        return run_with_pi_mono(command, description, mode).await;
    }

    debug!("Running standard implement with tool: {}", tool);
    // Fall through to standard leindex-core implement
    leindex_implement::run(command, description, session, tool, path, title).await
}

/// Pi-Mono execution mode
#[derive(Debug, Clone)]
enum PiMode {
    /// Single agent execution
    Single { agent: String },
    /// Chain execution (sequential)
    Chain { agents: Vec<String> },
    /// Parallel execution
    Parallel { agents: Vec<String> },
}

impl PiMode {
    /// Parse pi-mono flags into execution mode
    fn from_flags(
        pi_agent: Option<String>,
        pi_chain: Option<Vec<String>>,
        pi_parallel: Option<Vec<String>>,
    ) -> Option<Self> {
        if let Some(agent) = pi_agent {
            return Some(PiMode::Single { agent });
        }
        if let Some(agents) = pi_chain {
            return Some(PiMode::Chain { agents });
        }
        if let Some(agents) = pi_parallel {
            return Some(PiMode::Parallel { agents });
        }
        None
    }
}

/// Run implementation using Pi-Mono subagent system
async fn run_with_pi_mono(command: String, description: Vec<String>, mode: PiMode) -> Result<()> {
    info!("═══════════════════════════════════════════════════════════════");
    info!("  Pi-Mono Subagent Execution");
    info!("═══════════════════════════════════════════════════════════════");
    println!();

    // Load and validate configuration
    let config = load_config()?;

    if !config.enabled {
        anyhow::bail!("Pi-Mono is disabled. Run 'maestro configure --pi-mono' to enable it.");
    }

    if config.providers.is_empty() {
        anyhow::bail!(
            "No providers configured. Run 'maestro configure --pi-mono' to set up providers."
        );
    }
    if !config.providers.values().any(|p| p.is_configured) {
        anyhow::bail!(
            "No providers have valid credentials. Run 'maestro configure --pi-mono' to configure providers."
        );
    }

    if config.role_assignments.is_empty() {
        anyhow::bail!(
            "No agent role assignments configured. Run 'maestro configure --pi-mono' to set up agents."
        );
    }

    // Detect pi-mono
    let detection = PiDetection::detect()?;
    debug!("Found pi-mono at: {:?}", detection.executable_path);

    // Create runner
    let runner = SubagentRunner::from_detection(&detection)?;

    // Combine command and description into task
    let task = if description.is_empty() {
        command
    } else {
        format!("{}\n\nDescription: {}", command, description.join(" "))
    };

    // Execute based on mode
    match mode {
        PiMode::Single { agent } => {
            info!("Mode: Single Agent");
            info!("Agent: {}", agent);
            println!();
            info!("Task: {}", task);
            println!();

            let agent_type = parse_agent_type(&agent)?;
            let start = std::time::Instant::now();
            let result = runner.run(agent_type, &task, None::<&str>).await;
            let duration = start.elapsed();

            display_result(&result?, duration);
        }
        PiMode::Chain { agents } => {
            info!("Mode: Chain Execution");
            info!("Agents: {}", agents.join(" -> "));
            println!();
            info!("Task: {}", task);
            println!();

            execute_chain(&runner, &agents, &task).await?;
        }
        PiMode::Parallel { agents } => {
            info!("Mode: Parallel Execution");
            info!("Agents: {}", agents.join(", "));
            println!();
            info!("Task: {}", task);
            println!();

            execute_parallel(&runner, &agents, &task).await?;
        }
    }

    Ok(())
}

/// Execute agents in chain mode (sequential with output passing)
async fn execute_chain(
    runner: &SubagentRunner,
    agents: &[String],
    initial_task: &str,
) -> Result<()> {
    let mut current_output = initial_task.to_string();
    let mut total_duration = std::time::Duration::ZERO;

    for (i, agent) in agents.iter().enumerate() {
        info!("--- Step {}/{}: {} ---", i + 1, agents.len(), agent);

        let agent_type = parse_agent_type(agent)?;
        let start = std::time::Instant::now();

        // For chain mode, pass previous output as prompt
        let result = runner
            .run(agent_type, initial_task, Some(&current_output))
            .await?;
        let duration = start.elapsed();
        total_duration += duration;

        info!("Completed in {:?}", duration);
        println!();

        // Update for next iteration
        current_output = result.output.clone();
    }

    info!("Chain execution completed in {:?}", total_duration);
    Ok(())
}

/// Execute agents in parallel mode
async fn execute_parallel(runner: &SubagentRunner, agents: &[String], task: &str) -> Result<()> {
    use futures::future::join_all;

    let start = std::time::Instant::now();

    // Spawn all tasks in parallel
    let futures: Vec<_> = agents
        .iter()
        .map(|agent| {
            let agent = agent.clone();
            async move {
                let agent_start = std::time::Instant::now();
                let agent_type = parse_agent_type(&agent);
                let result = match agent_type {
                    Ok(t) => runner
                        .run(t, task, None::<&str>)
                        .await
                        .map_err(|e| anyhow::anyhow!(e)),
                    Err(e) => Err(e),
                };
                let duration = agent_start.elapsed();
                (agent, result, duration)
            }
        })
        .collect();

    // Wait for all to complete
    let results = join_all(futures).await;

    let total_duration = start.elapsed();

    // Display results
    info!("Parallel Execution Results:");
    println!();

    for (agent, result, duration) in results {
        match result {
            Ok(output) => {
                info!("✓ {} ({:?})", agent, duration);
                if !output.output.is_empty() {
                    let preview: String = output
                        .output
                        .lines()
                        .take(3)
                        .collect::<Vec<_>>()
                        .join("\n  ");
                    info!("  Preview: {}", preview);
                    if output.output.lines().count() > 3 {
                        info!("  ...");
                    }
                }
                println!();
            }
            Err(e) => {
                info!("✗ {} failed: {}", agent, e);
                println!();
            }
        }
    }

    info!("Parallel execution completed in {:?}", total_duration);
    Ok(())
}

/// Display execution result
fn display_result(result: &maestro_pi_mono::SubagentResult, duration: std::time::Duration) {
    info!("═══════════════════════════════════════════════════════════════");
    info!("  Execution Complete");
    info!("═══════════════════════════════════════════════════════════════");
    println!();
    info!(
        "Status: {}",
        if result.success {
            "✓ Success"
        } else {
            "✗ Failed"
        }
    );
    info!("Duration: {:?}", duration);
    println!();
    info!("Output:");
    info!("{}", result.output);
    println!();

    if let Some(ref error) = result.error {
        info!("Error: {}", error);
        println!();
    }

    if let Some(ref metrics) = result.usage {
        info!("Usage Metrics:");
        info!("  Input tokens: {}", metrics.tokens_input);
        info!("  Output tokens: {}", metrics.tokens_output);
        info!("  Total tokens: {}", metrics.tokens_total);
    }
}

/// Parse agent type string to PiAgentType
fn parse_agent_type(agent: &str) -> Result<PiAgentType> {
    match agent.to_lowercase().as_str() {
        "scout" => Ok(PiAgentType::Scout),
        "planner" | "architect" => Ok(PiAgentType::Planner),
        "reviewer" | "critic" => Ok(PiAgentType::Reviewer),
        "worker" | "kraken" => Ok(PiAgentType::Worker),
        _ => anyhow::bail!(
            "Unknown agent type: {}. Valid options: scout, planner, reviewer, worker",
            agent
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pi_mode_single() {
        let mode = PiMode::from_flags(Some("scout".to_string()), None, None);
        assert!(matches!(mode, Some(PiMode::Single { agent }) if agent == "scout"));
    }

    #[test]
    fn test_pi_mode_chain() {
        let mode = PiMode::from_flags(
            None,
            Some(vec!["scout".to_string(), "worker".to_string()]),
            None,
        );
        assert!(matches!(mode, Some(PiMode::Chain { .. })));
    }

    #[test]
    fn test_pi_mode_parallel() {
        let mode = PiMode::from_flags(
            None,
            None,
            Some(vec!["worker".to_string(), "worker".to_string()]),
        );
        assert!(matches!(mode, Some(PiMode::Parallel { .. })));
    }

    #[test]
    fn test_pi_mode_none() {
        let mode = PiMode::from_flags(None, None, None);
        assert!(mode.is_none());
    }

    #[test]
    fn test_parse_agent_type_scout() {
        assert_eq!(parse_agent_type("scout").unwrap(), PiAgentType::Scout);
        assert_eq!(parse_agent_type("SCOUT").unwrap(), PiAgentType::Scout);
    }

    #[test]
    fn test_parse_agent_type_planner() {
        assert_eq!(parse_agent_type("planner").unwrap(), PiAgentType::Planner);
        assert_eq!(parse_agent_type("architect").unwrap(), PiAgentType::Planner);
    }

    #[test]
    fn test_parse_agent_type_reviewer() {
        assert_eq!(parse_agent_type("reviewer").unwrap(), PiAgentType::Reviewer);
        assert_eq!(parse_agent_type("critic").unwrap(), PiAgentType::Reviewer);
    }

    #[test]
    fn test_parse_agent_type_worker() {
        assert_eq!(parse_agent_type("worker").unwrap(), PiAgentType::Worker);
        assert_eq!(parse_agent_type("kraken").unwrap(), PiAgentType::Worker);
    }

    #[test]
    fn test_parse_agent_type_invalid() {
        assert!(parse_agent_type("invalid").is_err());
    }
}
