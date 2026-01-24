//! Public API integration test for maestro-pi-mono
//!
//! This test verifies that the public API exports are accessible from external crates.

use maestro_pi_mono::{
    AgentRegistry, RegisteredAgent,
    AgentRole, PiAgentType, ToolAccess, TaskComplexity,
    ModelConfig,
    RoleAssignment,
};
use std::collections::HashMap;

#[test]
fn test_public_api_agent_registry() {
    // Test that we can create a registry using public API
    let mut role_assignments = HashMap::new();
    role_assignments.insert(
        "scout".to_string(),
        RoleAssignment {
            model_id: "claude-haiku-4-5".to_string(),
            provider: "anthropic".to_string(),
            fallback_models: None,
            use_reasoning: None,
        },
    );

    let config = ModelConfig {
        role_assignments,
        ..Default::default()
    };

    let registry = AgentRegistry::new(config);

    // Test that we can use the registry methods
    let roles = registry.registered_roles();
    assert!(!roles.is_empty());

    let agent = registry.get_agent(AgentRole::Scout).unwrap();
    assert_eq!(agent.role, AgentRole::Scout);
    assert_eq!(agent.pi_agent_type, PiAgentType::Scout);
}

#[test]
fn test_public_api_types() {
    // Test that all public types are accessible
    let _role = AgentRole::Scout;
    let _pi_type = PiAgentType::Scout;
    let _access = ToolAccess::new();
    let _complexity = TaskComplexity::Simple;

    // Test RegisteredAgent fields
    let agent = RegisteredAgent {
        role: AgentRole::Scout,
        pi_agent_type: PiAgentType::Scout,
        model_id: "test-model".to_string(),
        provider: "test-provider".to_string(),
        tool_access: ToolAccess::read_only(),
        complexity_range: (TaskComplexity::Trivial, TaskComplexity::Simple),
    };

    assert_eq!(agent.role, AgentRole::Scout);
    assert_eq!(agent.model_id, "test-model");
}

#[test]
fn test_public_api_from_config() {
    // Test creating registry from config with role assignments
    let mut role_assignments = HashMap::new();

    for role in &[AgentRole::Scout, AgentRole::Architect, AgentRole::Critic, AgentRole::Kraken] {
        let role_key = match role {
            AgentRole::Scout => "scout",
            AgentRole::Architect => "architect",
            AgentRole::Critic => "critic",
            AgentRole::Kraken => "kraken",
        };

        role_assignments.insert(
            role_key.to_string(),
            RoleAssignment {
                model_id: "claude-sonnet-4-5".to_string(),
                provider: "anthropic".to_string(),
                fallback_models: None,
                use_reasoning: None,
            },
        );
    }

    let config = ModelConfig {
        role_assignments,
        ..Default::default()
    };

    let registry = AgentRegistry::new(config);

    // Verify all roles are registered
    let roles = registry.registered_roles();
    assert_eq!(roles.len(), 4);

    // Verify we can get model assignments
    for role in &[AgentRole::Scout, AgentRole::Architect, AgentRole::Critic, AgentRole::Kraken] {
        let model = registry.get_model_for_role(role.clone()).unwrap();
        assert_eq!(model, "claude-sonnet-4-5");
    }
}
