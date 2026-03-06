// Verify public API exports
use std::collections::HashSet;

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

    // Verify mapping helpers are accessible and coherent.
    let tool_access = ToolAccess {
        can_read: true,
        can_write: false,
        can_execute: false,
        can_search: true,
        allowed_tools: {
            let mut tools = HashSet::new();
            tools.insert("grep".to_string());
            tools
        },
    };
    let mapping = AgentMapping::new(
        AgentRole::Scout,
        PiAgentType::Scout,
        tool_access,
        (TaskComplexity::Trivial, TaskComplexity::Simple),
        "test mapping".to_string(),
    );
    assert!(mapping.can_handle_complexity(TaskComplexity::Simple));
    assert_eq!(
        role_to_pi_agent_type(&AgentRole::Scout),
        Some(PiAgentType::Scout)
    );
    assert!(!default_agent_mappings().is_empty());

    // Verify functions are accessible
    let _presets = default_presets();
    let _preset = get_preset("implement");
    let _names = preset_names();

    println!("All public API exports are accessible!");
}
