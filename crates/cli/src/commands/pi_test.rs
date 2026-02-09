//! Pi-Test command for Maestro
//!
//! This module provides the `maestro pi-test` command which executes
//! test subagent tasks using the SubagentRunner and displays results.

use anyhow::Result;
use maestro_pi_mono::{
    load_config, PiDetection, SubagentRunner,
};
use maestro_pi_mono::agents::mapping::PiAgentType;
use tracing::debug;
use std::time::Duration;

/// Run the pi-test command
///
/// Executes a test subagent task and displays results and diagnostics.
pub async fn run(
    task: String,
    agent_type: Option<String>,
    timeout_secs: Option<u64>,
    verbose: bool,
) -> Result<()> {
    // Validate task is not empty
    let task = task.trim();
    if task.is_empty() {
        anyhow::bail!("Task cannot be empty. Please provide a task description.");
    }

    debug!("Running pi-test command with task: {}", task);

    // Load configuration
    let config = load_config()?;

    // Validate configuration
    if !config.enabled {
        anyhow::bail!("Pi-Mono is disabled. Run 'maestro configure --pi-mono' to enable it.");
    }

    // Validate providers are configured
    if config.providers.is_empty() {
        anyhow::bail!("No providers configured. Run 'maestro configure --pi-mono' to set up providers.");
    }
    if !config.providers.values().any(|p| p.is_configured) {
        anyhow::bail!("No providers have valid credentials. Run 'maestro configure --pi-mono' to configure providers.");
    }

    // Validate role assignments exist
    if config.role_assignments.is_empty() {
        anyhow::bail!("No agent role assignments configured. Run 'maestro configure --pi-mono' to set up agents.");
    }

    // Detect pi-mono
    let detection = PiDetection::detect()?;
    debug!("Found pi-mono at: {:?}", detection.executable_path);

    // Determine agent type
    let agent_type = parse_agent_type(agent_type)?;

    // Create runner
    let runner = SubagentRunner::from_detection(&detection)?;

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("  Pi-Mono Test Execution");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("Agent Type: {:?}", agent_type);
    println!("Task: {}", task);
    println!();

    // Execute the task
    println!("Executing...");
    let start = std::time::Instant::now();

    let result = if let Some(secs) = timeout_secs {
        let timeout = Duration::from_secs(secs);
        tokio::time::timeout(timeout, runner.run(agent_type, &task, None::<&str>)).await
    } else {
        Ok(runner.run(agent_type, &task, None::<&str>).await)
    };

    let duration = start.elapsed();

    match result {
        Ok(Ok(output)) => {
            println!();
            println!("✓ Execution successful");
            println!("Duration: {:?}", duration);
            println!();
            println!("Output:");
            println!("{}", output.output);
            println!();

            if verbose {
                print_diagnostics(&output, duration);
            }
        }
        Ok(Err(e)) => {
            println!();
            println!("✗ Execution failed");
            println!("Duration: {:?}", duration);
            println!();
            println!("Error: {}", e);
            println!();

            if verbose {
                println!("Diagnostics:");
                println!("  - Check if pi-mono is properly installed");
                println!("  - Verify provider API keys are set");
                println!("  - Ensure the selected model is available");
                println!();
            }

            return Err(e.into());
        }
        Err(_) => {
            println!();
            println!("✗ Execution timed out");
            println!("Duration: {:?}", duration);
            println!();

            if verbose {
                println!("Diagnostics:");
                println!("  - Task exceeded the timeout limit");
                println!("  - Try with a longer timeout using --timeout");
                println!("  - Consider breaking the task into smaller steps");
                println!();
            }

            anyhow::bail!("Task execution timed out after {:?}", duration);
        }
    }

    Ok(())
}

/// Parse agent type from string
fn parse_agent_type(agent_type: Option<String>) -> Result<PiAgentType> {
    match agent_type.as_deref() {
        None | Some("scout") => Ok(PiAgentType::Scout),
        Some("planner") | Some("architect") => Ok(PiAgentType::Planner),
        Some("reviewer") | Some("critic") => Ok(PiAgentType::Reviewer),
        Some("worker") | Some("kraken") => Ok(PiAgentType::Worker),
        Some(other) => anyhow::bail!("Unknown agent type: {}. Valid options: scout, planner, reviewer, worker", other),
    }
}

/// Print detailed diagnostics
fn print_diagnostics(result: &maestro_pi_mono::SubagentResult, duration: std::time::Duration) {
    println!("Diagnostics:");
    println!("  Duration: {:?}", duration);
    println!("  Exit code: {:?}", result.exit_code);

    // Print error if present
    if let Some(ref error) = result.error {
        println!("  Error: {}", error);
    }

    // Print events if available
    if !result.events.is_empty() {
        println!("  Events: {} captured", result.events.len());
        if result.events.len() <= 5 {
            for (i, event) in result.events.iter().enumerate() {
                println!("    [{}]: {:?}", i, event.event_type);
            }
        } else {
            println!("    First 5 events:");
            for (i, event) in result.events.iter().take(5).enumerate() {
                println!("    [{}]: {:?}", i, event.event_type);
            }
        }
    }

    if let Some(metrics) = &result.usage {
        println!("  Usage metrics:");
        println!("    Input tokens: {}", metrics.tokens_input);
        println!("    Output tokens: {}", metrics.tokens_output);
        println!("    Total tokens: {}", metrics.tokens_total);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that pi-test command exists and compiles
    #[test]
    fn test_pi_test_command_exists() {
        assert!(true);
    }

    /// Test parsing agent type from string
    #[test]
    fn test_parse_agent_type_scout() {
        assert_eq!(parse_agent_type(Some("scout".to_string())).unwrap(), PiAgentType::Scout);
        assert_eq!(parse_agent_type(None).unwrap(), PiAgentType::Scout);
    }

    /// Test parsing agent type - planner
    #[test]
    fn test_parse_agent_type_planner() {
        assert_eq!(parse_agent_type(Some("planner".to_string())).unwrap(), PiAgentType::Planner);
        assert_eq!(parse_agent_type(Some("architect".to_string())).unwrap(), PiAgentType::Planner);
    }

    /// Test parsing agent type - reviewer
    #[test]
    fn test_parse_agent_type_reviewer() {
        assert_eq!(parse_agent_type(Some("reviewer".to_string())).unwrap(), PiAgentType::Reviewer);
        assert_eq!(parse_agent_type(Some("critic".to_string())).unwrap(), PiAgentType::Reviewer);
    }

    /// Test parsing agent type - worker
    #[test]
    fn test_parse_agent_type_worker() {
        assert_eq!(parse_agent_type(Some("worker".to_string())).unwrap(), PiAgentType::Worker);
        assert_eq!(parse_agent_type(Some("kraken".to_string())).unwrap(), PiAgentType::Worker);
    }

    /// Test parsing agent type - invalid
    #[test]
    fn test_parse_agent_type_invalid() {
        let result = parse_agent_type(Some("invalid".to_string()));
        assert!(result.is_err());
    }

    /// Test print diagnostics with empty result
    #[test]
    fn test_print_diagnostics_empty() {
        let result = maestro_pi_mono::SubagentResult {
            success: true,
            task: "test".to_string(),
            agent: "test_agent".to_string(),
            agent_type: "scout".to_string(),
            output: String::new(),
            error: None,
            exit_code: Some(0),
            duration: std::time::Duration::from_millis(100),
            usage: None,
            events: vec![],
        };
        print_diagnostics(&result, std::time::Duration::from_millis(100));
    }
}
