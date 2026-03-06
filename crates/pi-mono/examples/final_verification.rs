// Final Verification Program
use maestro_pi_mono::{
    default_presets, get_preset, preset_names, AgentRole, PiAgentType, WorkflowMode,
    WorkflowPreset, WorkflowStep,
};

fn main() {
    println!("=== FINAL VERIFICATION ===\n");

    // 1. Verify preset_names
    println!("✓ preset_names() returns:");
    for name in preset_names() {
        println!("  - {}", name);
    }
    println!();

    // 2. Verify default_presets count
    let presets = default_presets();
    assert_eq!(presets.len(), 3, "Should have 3 presets");
    println!("✓ default_presets() returns 3 presets\n");

    // 3. Verify /implement preset
    let implement = get_preset("implement").expect("implement preset exists");
    assert_eq!(implement.name, "implement");
    assert_eq!(implement.mode, WorkflowMode::Chain);
    assert_eq!(implement.step_count(), 3);
    assert_eq!(implement.steps[0].role, AgentRole::Scout);
    assert_eq!(implement.steps[1].role, AgentRole::Architect);
    assert_eq!(implement.steps[2].role, AgentRole::Kraken);
    assert!(implement.steps[0].depends_on_previous);
    assert!(implement.steps[1].depends_on_previous);
    assert!(implement.steps[2].depends_on_previous);
    println!("✓ /implement preset verified:");
    println!("  Mode: {:?}", implement.mode);
    println!(
        "  Steps: {:?}",
        implement.steps.iter().map(|s| &s.role).collect::<Vec<_>>()
    );
    println!();

    // 4. Verify /implement-and-review preset
    let implement_review = get_preset("implement-and-review").expect("preset exists");
    assert_eq!(implement_review.name, "implement-and-review");
    assert_eq!(implement_review.mode, WorkflowMode::Chain);
    assert_eq!(implement_review.step_count(), 3);
    assert_eq!(implement_review.steps[0].role, AgentRole::Kraken);
    assert_eq!(implement_review.steps[1].role, AgentRole::Critic);
    assert_eq!(implement_review.steps[2].role, AgentRole::Kraken);
    println!("✓ /implement-and-review preset verified:");
    println!("  Mode: {:?}", implement_review.mode);
    println!(
        "  Steps: {:?}",
        implement_review
            .steps
            .iter()
            .map(|s| &s.role)
            .collect::<Vec<_>>()
    );
    println!();

    // 5. Verify /parallel-review preset
    let parallel = get_preset("parallel-review").expect("preset exists");
    assert_eq!(parallel.name, "parallel-review");
    assert_eq!(parallel.mode, WorkflowMode::Parallel);
    assert_eq!(parallel.step_count(), 3);
    assert!(parallel.steps.iter().all(|s| s.role == AgentRole::Critic));
    assert!(!parallel.steps[0].depends_on_previous);
    assert!(!parallel.steps[1].depends_on_previous);
    assert!(!parallel.steps[2].depends_on_previous);
    println!("✓ /parallel-review preset verified:");
    println!("  Mode: {:?}", parallel.mode);
    println!(
        "  Steps: {:?}",
        parallel.steps.iter().map(|s| &s.role).collect::<Vec<_>>()
    );
    println!();

    // 6. Verify helper methods
    assert!(implement.is_chained());
    assert!(!implement.is_parallel());
    assert!(!implement.is_single());
    assert!(parallel.is_parallel());
    assert!(!parallel.is_chained());
    println!("✓ Helper methods work correctly\n");

    // 7. Verify WorkflowStep helpers
    let chained = WorkflowStep::chained(AgentRole::Scout, PiAgentType::Scout);
    assert!(chained.depends_on_previous);
    let independent = WorkflowStep::independent(AgentRole::Critic, PiAgentType::Reviewer);
    assert!(!independent.depends_on_previous);
    println!("✓ WorkflowStep helper methods work\n");

    println!("=== ALL VERIFICATIONS PASSED ===");
}
