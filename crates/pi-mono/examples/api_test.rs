// Verify public API exports
use maestro_pi_mono::{
    default_mappings as default_agent_mappings,
    default_presets,
    get_preset,
    preset_names,
    role_to_pi_agent_type,
    AgentMapping,
    // Mapping types
    AgentRole,
    PiAgentType,
    TaskComplexity,
    ToolAccess,
    // Workflow types
    WorkflowMode,
    WorkflowPreset,
    WorkflowStep,
};

fn main() {
    // Verify AgentRole enum
    let _role = AgentRole::Scout;
    let _role = AgentRole::Architect;
    let _role = AgentRole::Critic;
    let _role = AgentRole::Kraken;

    // Verify PiAgentType enum
    let _pi_type = PiAgentType::Scout;
    let _pi_type = PiAgentType::Planner;
    let _pi_type = PiAgentType::Reviewer;
    let _pi_type = PiAgentType::Worker;

    // Verify WorkflowMode enum
    let _mode = WorkflowMode::Single;
    let _mode = WorkflowMode::Parallel;
    let _mode = WorkflowMode::Chain;

    // Verify WorkflowStep can be created
    let _step = WorkflowStep::new(
        AgentRole::Scout,
        PiAgentType::Scout,
        Some("test".to_string()),
        true,
    );

    // Verify WorkflowPreset can be created
    let _preset = WorkflowPreset::new(
        "test".to_string(),
        "test preset".to_string(),
        WorkflowMode::Chain,
        vec![],
    );

    // Verify functions are accessible
    let _presets = default_presets();
    let _preset = get_preset("implement");
    let _names = preset_names();

    println!("All public API exports are accessible!");
}
