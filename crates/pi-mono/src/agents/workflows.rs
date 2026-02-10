//! # Workflow presets for Maestro Pi-Mono integration
//!
//! This module provides predefined workflow presets for common agent orchestration patterns.
//!
//! ## Workflow Modes
//!
//! - **Single**: Execute a single agent
//! - **Parallel**: Execute multiple agents in parallel
//! - **Chain**: Execute agents sequentially, with each step depending on the previous
//!
//! ## Default Presets
//!
//! - `implement`: Chain mode - scout -> architect -> kraken
//! - `implement-and-review`: Chain mode - kraken -> critic -> kraken
//! - `parallel-review`: Parallel mode - multiple critics

use crate::agents::mapping::{AgentRole, PiAgentType};
use serde::{Deserialize, Serialize};

/// Workflow execution mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum WorkflowMode {
    /// Execute a single agent
    Single,
    /// Execute multiple agents in parallel
    Parallel,
    /// Execute agents sequentially, with each step depending on the previous
    Chain,
}

/// A single workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// The agent role for this step
    pub role: AgentRole,
    /// The Pi-Mono agent type for this step
    pub pi_agent_type: PiAgentType,
    /// Optional custom prompt for this step
    pub prompt: Option<String>,
    /// Whether this step depends on the previous step's output (for chain mode)
    pub depends_on_previous: bool,
}

impl WorkflowStep {
    /// Create a new workflow step
    pub fn new(
        role: AgentRole,
        pi_agent_type: PiAgentType,
        prompt: Option<String>,
        depends_on_previous: bool,
    ) -> Self {
        Self {
            role,
            pi_agent_type,
            prompt,
            depends_on_previous,
        }
    }

    /// Create a step that depends on the previous step
    pub fn chained(role: AgentRole, pi_agent_type: PiAgentType) -> Self {
        Self {
            role,
            pi_agent_type,
            prompt: None,
            depends_on_previous: true,
        }
    }

    /// Create a step that does not depend on the previous step
    pub fn independent(role: AgentRole, pi_agent_type: PiAgentType) -> Self {
        Self {
            role,
            pi_agent_type,
            prompt: None,
            depends_on_previous: false,
        }
    }
}

/// Workflow preset definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPreset {
    /// Name of the preset
    pub name: String,
    /// Description of what this preset does
    pub description: String,
    /// Execution mode for this preset
    pub mode: WorkflowMode,
    /// Steps in the workflow
    pub steps: Vec<WorkflowStep>,
}

impl WorkflowPreset {
    /// Create a new workflow preset
    pub fn new(
        name: String,
        description: String,
        mode: WorkflowMode,
        steps: Vec<WorkflowStep>,
    ) -> Self {
        Self {
            name,
            description,
            mode,
            steps,
        }
    }

    /// Get the number of steps in this workflow
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Check if this preset uses chain mode
    pub fn is_chained(&self) -> bool {
        self.mode == WorkflowMode::Chain
    }

    /// Check if this preset uses parallel mode
    pub fn is_parallel(&self) -> bool {
        self.mode == WorkflowMode::Parallel
    }

    /// Check if this preset uses single mode
    pub fn is_single(&self) -> bool {
        self.mode == WorkflowMode::Single
    }
}

/// Get default workflow presets
pub fn default_presets() -> Vec<WorkflowPreset> {
    vec![
        // /implement: scout -> architect -> kraken (chain mode)
        WorkflowPreset {
            name: "implement".to_string(),
            description: "Implementation workflow: scout -> architect -> kraken".to_string(),
            mode: WorkflowMode::Chain,
            steps: vec![
                WorkflowStep::chained(AgentRole::Scout, PiAgentType::Scout),
                WorkflowStep::chained(AgentRole::Architect, PiAgentType::Planner),
                WorkflowStep::chained(AgentRole::Kraken, PiAgentType::Worker),
            ],
        },
        // /implement-and-review: kraken -> critic -> kraken (chain mode)
        WorkflowPreset {
            name: "implement-and-review".to_string(),
            description: "Implementation with review: kraken -> critic -> kraken".to_string(),
            mode: WorkflowMode::Chain,
            steps: vec![
                WorkflowStep::chained(AgentRole::Kraken, PiAgentType::Worker),
                WorkflowStep::chained(AgentRole::Critic, PiAgentType::Reviewer),
                WorkflowStep::chained(AgentRole::Kraken, PiAgentType::Worker),
            ],
        },
        // /parallel-review: parallel critic execution
        WorkflowPreset {
            name: "parallel-review".to_string(),
            description: "Parallel code review with multiple critics".to_string(),
            mode: WorkflowMode::Parallel,
            steps: vec![
                WorkflowStep::independent(AgentRole::Critic, PiAgentType::Reviewer),
                WorkflowStep::independent(AgentRole::Critic, PiAgentType::Reviewer),
                WorkflowStep::independent(AgentRole::Critic, PiAgentType::Reviewer),
            ],
        },
    ]
}

/// Get a preset by name
pub fn get_preset(name: &str) -> Option<WorkflowPreset> {
    default_presets().into_iter().find(|p| p.name == name)
}

/// Available preset names
pub fn preset_names() -> &'static [&'static str] {
    &["implement", "implement-and-review", "parallel-review"]
}

#[cfg(test)]
mod tests {
    use super::*;

    // WorkflowMode enum tests
    #[test]
    fn test_workflow_mode_variants() {
        let single = WorkflowMode::Single;
        let parallel = WorkflowMode::Parallel;
        let chain = WorkflowMode::Chain;

        assert_eq!(single, WorkflowMode::Single);
        assert_eq!(parallel, WorkflowMode::Parallel);
        assert_eq!(chain, WorkflowMode::Chain);
    }

    #[test]
    fn test_workflow_mode_equality() {
        assert_eq!(WorkflowMode::Single, WorkflowMode::Single);
        assert_ne!(WorkflowMode::Single, WorkflowMode::Parallel);
        assert_ne!(WorkflowMode::Parallel, WorkflowMode::Chain);
    }

    #[test]
    fn test_workflow_mode_hashable() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(WorkflowMode::Single);
        set.insert(WorkflowMode::Parallel);
        set.insert(WorkflowMode::Chain);
        assert_eq!(set.len(), 3);
    }

    // WorkflowStep struct tests
    #[test]
    fn test_workflow_step_new() {
        let step = WorkflowStep::new(
            AgentRole::Scout,
            PiAgentType::Scout,
            Some("Custom prompt".to_string()),
            true,
        );

        assert_eq!(step.role, AgentRole::Scout);
        assert_eq!(step.pi_agent_type, PiAgentType::Scout);
        assert_eq!(step.prompt, Some("Custom prompt".to_string()));
        assert!(step.depends_on_previous);
    }

    #[test]
    fn test_workflow_step_chained() {
        let step = WorkflowStep::chained(AgentRole::Architect, PiAgentType::Planner);

        assert_eq!(step.role, AgentRole::Architect);
        assert_eq!(step.pi_agent_type, PiAgentType::Planner);
        assert!(step.depends_on_previous);
        assert!(step.prompt.is_none());
    }

    #[test]
    fn test_workflow_step_independent() {
        let step = WorkflowStep::independent(AgentRole::Critic, PiAgentType::Reviewer);

        assert_eq!(step.role, AgentRole::Critic);
        assert_eq!(step.pi_agent_type, PiAgentType::Reviewer);
        assert!(!step.depends_on_previous);
        assert!(step.prompt.is_none());
    }

    #[test]
    fn test_workflow_step_serialization() {
        let step = WorkflowStep::new(
            AgentRole::Kraken,
            PiAgentType::Worker,
            Some("Test prompt".to_string()),
            false,
        );

        let serialized = serde_json::to_string(&step).unwrap();
        let deserialized: WorkflowStep = serde_json::from_str(&serialized).unwrap();

        assert_eq!(step.role, deserialized.role);
        assert_eq!(step.pi_agent_type, deserialized.pi_agent_type);
        assert_eq!(step.prompt, deserialized.prompt);
        assert_eq!(step.depends_on_previous, deserialized.depends_on_previous);
    }

    // WorkflowPreset struct tests
    #[test]
    fn test_workflow_preset_new() {
        let preset = WorkflowPreset::new(
            "test-preset".to_string(),
            "A test preset".to_string(),
            WorkflowMode::Chain,
            vec![WorkflowStep::chained(AgentRole::Scout, PiAgentType::Scout)],
        );

        assert_eq!(preset.name, "test-preset");
        assert_eq!(preset.description, "A test preset");
        assert_eq!(preset.mode, WorkflowMode::Chain);
        assert_eq!(preset.step_count(), 1);
    }

    #[test]
    fn test_workflow_preset_step_count() {
        let preset = WorkflowPreset::new(
            "multi-step".to_string(),
            "Multiple steps".to_string(),
            WorkflowMode::Chain,
            vec![
                WorkflowStep::chained(AgentRole::Scout, PiAgentType::Scout),
                WorkflowStep::chained(AgentRole::Architect, PiAgentType::Planner),
                WorkflowStep::chained(AgentRole::Kraken, PiAgentType::Worker),
            ],
        );

        assert_eq!(preset.step_count(), 3);
    }

    #[test]
    fn test_workflow_preset_is_chained() {
        let preset = WorkflowPreset::new(
            "chained".to_string(),
            "Chained workflow".to_string(),
            WorkflowMode::Chain,
            vec![],
        );

        assert!(preset.is_chained());
        assert!(!preset.is_parallel());
        assert!(!preset.is_single());
    }

    #[test]
    fn test_workflow_preset_is_parallel() {
        let preset = WorkflowPreset::new(
            "parallel".to_string(),
            "Parallel workflow".to_string(),
            WorkflowMode::Parallel,
            vec![],
        );

        assert!(!preset.is_chained());
        assert!(preset.is_parallel());
        assert!(!preset.is_single());
    }

    #[test]
    fn test_workflow_preset_is_single() {
        let preset = WorkflowPreset::new(
            "single".to_string(),
            "Single agent workflow".to_string(),
            WorkflowMode::Single,
            vec![],
        );

        assert!(!preset.is_chained());
        assert!(!preset.is_parallel());
        assert!(preset.is_single());
    }

    #[test]
    fn test_workflow_preset_serialization() {
        let preset = WorkflowPreset::new(
            "serialize-test".to_string(),
            "Test serialization".to_string(),
            WorkflowMode::Chain,
            vec![WorkflowStep::chained(AgentRole::Scout, PiAgentType::Scout)],
        );

        let serialized = serde_json::to_string(&preset).unwrap();
        let deserialized: WorkflowPreset = serde_json::from_str(&serialized).unwrap();

        assert_eq!(preset.name, deserialized.name);
        assert_eq!(preset.description, deserialized.description);
        assert_eq!(preset.mode, deserialized.mode);
        assert_eq!(preset.step_count(), deserialized.step_count());
    }

    // Default presets tests
    #[test]
    fn test_default_presets_count() {
        let presets = default_presets();
        assert_eq!(presets.len(), 3);
    }

    #[test]
    fn test_default_presets_implement() {
        let presets = default_presets();
        let implement = presets
            .iter()
            .find(|p| p.name == "implement")
            .expect("implement preset should exist");

        assert_eq!(implement.name, "implement");
        assert!(implement.description.contains("scout"));
        assert!(implement.description.contains("architect"));
        assert!(implement.description.contains("kraken"));
        assert_eq!(implement.mode, WorkflowMode::Chain);
        assert_eq!(implement.step_count(), 3);
    }

    #[test]
    fn test_default_presets_implement_steps() {
        let presets = default_presets();
        let implement = presets
            .iter()
            .find(|p| p.name == "implement")
            .expect("implement preset should exist");

        assert_eq!(implement.steps[0].role, AgentRole::Scout);
        assert_eq!(implement.steps[0].pi_agent_type, PiAgentType::Scout);
        assert!(implement.steps[0].depends_on_previous);

        assert_eq!(implement.steps[1].role, AgentRole::Architect);
        assert_eq!(implement.steps[1].pi_agent_type, PiAgentType::Planner);
        assert!(implement.steps[1].depends_on_previous);

        assert_eq!(implement.steps[2].role, AgentRole::Kraken);
        assert_eq!(implement.steps[2].pi_agent_type, PiAgentType::Worker);
        assert!(implement.steps[2].depends_on_previous);
    }

    #[test]
    fn test_default_presets_implement_and_review() {
        let presets = default_presets();
        let implement_review = presets
            .iter()
            .find(|p| p.name == "implement-and-review")
            .expect("implement-and-review preset should exist");

        assert_eq!(implement_review.name, "implement-and-review");
        assert!(implement_review.description.contains("kraken"));
        assert!(implement_review.description.contains("critic"));
        assert_eq!(implement_review.mode, WorkflowMode::Chain);
        assert_eq!(implement_review.step_count(), 3);
    }

    #[test]
    fn test_default_presets_implement_and_review_steps() {
        let presets = default_presets();
        let implement_review = presets
            .iter()
            .find(|p| p.name == "implement-and-review")
            .expect("implement-and-review preset should exist");

        assert_eq!(implement_review.steps[0].role, AgentRole::Kraken);
        assert_eq!(implement_review.steps[0].pi_agent_type, PiAgentType::Worker);
        assert!(implement_review.steps[0].depends_on_previous);

        assert_eq!(implement_review.steps[1].role, AgentRole::Critic);
        assert_eq!(
            implement_review.steps[1].pi_agent_type,
            PiAgentType::Reviewer
        );
        assert!(implement_review.steps[1].depends_on_previous);

        assert_eq!(implement_review.steps[2].role, AgentRole::Kraken);
        assert_eq!(implement_review.steps[2].pi_agent_type, PiAgentType::Worker);
        assert!(implement_review.steps[2].depends_on_previous);
    }

    #[test]
    fn test_default_presets_parallel_review() {
        let presets = default_presets();
        let parallel_review = presets
            .iter()
            .find(|p| p.name == "parallel-review")
            .expect("parallel-review preset should exist");

        assert_eq!(parallel_review.name, "parallel-review");
        assert!(parallel_review.description.contains("Parallel"));
        assert!(parallel_review.description.contains("critic"));
        assert_eq!(parallel_review.mode, WorkflowMode::Parallel);
        assert_eq!(parallel_review.step_count(), 3);
    }

    #[test]
    fn test_default_presets_parallel_review_steps() {
        let presets = default_presets();
        let parallel_review = presets
            .iter()
            .find(|p| p.name == "parallel-review")
            .expect("parallel-review preset should exist");

        // All steps should be critics
        for step in &parallel_review.steps {
            assert_eq!(step.role, AgentRole::Critic);
            assert_eq!(step.pi_agent_type, PiAgentType::Reviewer);
            assert!(!step.depends_on_previous);
        }
    }

    // get_preset function tests
    #[test]
    fn test_get_preset_implement() {
        let preset = get_preset("implement");
        assert!(preset.is_some());

        let preset = preset.unwrap();
        assert_eq!(preset.name, "implement");
        assert_eq!(preset.mode, WorkflowMode::Chain);
        assert_eq!(preset.step_count(), 3);
    }

    #[test]
    fn test_get_preset_implement_and_review() {
        let preset = get_preset("implement-and-review");
        assert!(preset.is_some());

        let preset = preset.unwrap();
        assert_eq!(preset.name, "implement-and-review");
        assert_eq!(preset.mode, WorkflowMode::Chain);
        assert_eq!(preset.step_count(), 3);
    }

    #[test]
    fn test_get_preset_parallel_review() {
        let preset = get_preset("parallel-review");
        assert!(preset.is_some());

        let preset = preset.unwrap();
        assert_eq!(preset.name, "parallel-review");
        assert_eq!(preset.mode, WorkflowMode::Parallel);
        assert_eq!(preset.step_count(), 3);
    }

    #[test]
    fn test_get_preset_nonexistent() {
        let preset = get_preset("nonexistent-preset");
        assert!(preset.is_none());
    }

    // preset_names function tests
    #[test]
    fn test_preset_names() {
        let names = preset_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"implement"));
        assert!(names.contains(&"implement-and-review"));
        assert!(names.contains(&"parallel-review"));
    }

    #[test]
    fn test_preset_names_static() {
        let names1 = preset_names();
        let names2 = preset_names();
        assert_eq!(names1, names2);
    }

    // Integration tests
    #[test]
    fn test_all_presets_have_valid_names() {
        let names = preset_names();
        for name in names {
            let preset = get_preset(name);
            assert!(
                preset.is_some(),
                "Preset name '{}' should exist in get_preset",
                name
            );
        }
    }

    #[test]
    fn test_all_default_presets_match_names() {
        let presets = default_presets();
        let names = preset_names();

        assert_eq!(presets.len(), names.len());

        for preset in &presets {
            assert!(
                names.contains(&preset.name.as_str()),
                "Preset '{}' should be in preset_names",
                preset.name
            );
        }
    }

    #[test]
    fn test_chain_presets_all_depend_on_previous() {
        let presets = default_presets();

        for preset in &presets {
            if preset.mode == WorkflowMode::Chain {
                for step in &preset.steps {
                    assert!(
                        step.depends_on_previous,
                        "Chain mode preset '{}' should have all steps marked as depends_on_previous",
                        preset.name
                    );
                }
            }
        }
    }

    #[test]
    fn test_parallel_presets_no_dependence_on_previous() {
        let presets = default_presets();

        for preset in &presets {
            if preset.mode == WorkflowMode::Parallel {
                for step in &preset.steps {
                    assert!(
                        !step.depends_on_previous,
                        "Parallel mode preset '{}' should have all steps marked as independent",
                        preset.name
                    );
                }
            }
        }
    }

    #[test]
    fn test_workflow_step_matches_role_and_agent_type() {
        let presets = default_presets();

        for preset in &presets {
            for step in &preset.steps {
                // Verify the role and agent type mapping matches the default mappings
                let expected_type = match step.role {
                    AgentRole::Scout => PiAgentType::Scout,
                    AgentRole::Architect => PiAgentType::Planner,
                    AgentRole::Critic => PiAgentType::Reviewer,
                    AgentRole::Kraken => PiAgentType::Worker,
                };

                assert_eq!(
                    step.pi_agent_type, expected_type,
                    "Step role {:?} should map to agent type {:?} in preset '{}'",
                    step.role, expected_type, preset.name
                );
            }
        }
    }
}
