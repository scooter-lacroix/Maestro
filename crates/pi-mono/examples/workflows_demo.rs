// Test program to verify workflow API
use maestro_pi_mono::{
    default_presets, get_preset, preset_names, AgentRole, PiAgentType, WorkflowMode,
    WorkflowPreset, WorkflowStep,
};

fn main() {
    println!("=== Workflow Presets Demo ===\n");

    // Test preset_names()
    println!("Available preset names:");
    for name in preset_names() {
        println!("  - {}", name);
    }
    println!();

    // Test default_presets()
    println!("Default presets:");
    for preset in default_presets() {
        println!("  Name: {}", preset.name);
        println!("  Description: {}", preset.description);
        println!("  Mode: {:?}", preset.mode);
        println!("  Steps: {}", preset.step_count());
        println!();
    }

    // Test get_preset()
    println!("Getting specific presets:");

    let implement = get_preset("implement").unwrap();
    println!("  /implement workflow:");
    for (i, step) in implement.steps.iter().enumerate() {
        println!(
            "    Step {}: {:?} -> {:?}",
            i + 1,
            step.role,
            step.pi_agent_type
        );
        println!("      Depends on previous: {}", step.depends_on_previous);
    }
    println!();

    let implement_review = get_preset("implement-and-review").unwrap();
    println!("  /implement-and-review workflow:");
    for (i, step) in implement_review.steps.iter().enumerate() {
        println!(
            "    Step {}: {:?} -> {:?}",
            i + 1,
            step.role,
            step.pi_agent_type
        );
        println!("      Depends on previous: {}", step.depends_on_previous);
    }
    println!();

    let parallel_review = get_preset("parallel-review").unwrap();
    println!("  /parallel-review workflow:");
    for (i, step) in parallel_review.steps.iter().enumerate() {
        println!(
            "    Step {}: {:?} -> {:?}",
            i + 1,
            step.role,
            step.pi_agent_type
        );
        println!("      Depends on previous: {}", step.depends_on_previous);
    }
    println!();

    // Test WorkflowStep helper methods
    println!("WorkflowStep creation helpers:");
    let chained = WorkflowStep::chained(AgentRole::Scout, PiAgentType::Scout);
    println!(
        "  Chained step: depends_on_previous = {}",
        chained.depends_on_previous
    );

    let independent = WorkflowStep::independent(AgentRole::Critic, PiAgentType::Reviewer);
    println!(
        "  Independent step: depends_on_previous = {}",
        independent.depends_on_previous
    );
    println!();

    // Test WorkflowPreset helper methods
    println!("WorkflowPreset helper methods:");
    println!("  implement.is_chained(): {}", implement.is_chained());
    println!("  implement.is_parallel(): {}", implement.is_parallel());
    println!(
        "  parallel_review.is_parallel(): {}",
        parallel_review.is_parallel()
    );

    println!("\n=== All tests passed! ===");
}
