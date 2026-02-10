//! # Agent mapping structures for Maestro to Pi-Mono integration
//!
//! This module provides data structures for mapping Maestro agent roles to Pi-Mono agent types,
//! including tool access permissions and task complexity levels.

use crate::{
    config::models::PiMonoConfig,
    detection::PiDetection,
    error::{Error, Result},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Maestro agent roles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentRole {
    Scout,
    Architect,
    Critic,
    Kraken,
}

/// Pi-mono agent types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PiAgentType {
    Scout,
    Planner,
    Reviewer,
    Worker,
}

/// Tool access permissions for agents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolAccess {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
    pub can_search: bool,
    pub allowed_tools: HashSet<String>,
}

impl ToolAccess {
    /// Create a new ToolAccess with default permissions
    pub fn new() -> Self {
        Self {
            can_read: false,
            can_write: false,
            can_execute: false,
            can_search: false,
            allowed_tools: HashSet::new(),
        }
    }

    /// Create a new ToolAccess with read-only access
    pub fn read_only() -> Self {
        Self {
            can_read: true,
            can_write: false,
            can_execute: false,
            can_search: true,
            allowed_tools: HashSet::new(),
        }
    }

    /// Create a new ToolAccess with full access
    pub fn full_access() -> Self {
        Self {
            can_read: true,
            can_write: true,
            can_execute: true,
            can_search: true,
            allowed_tools: HashSet::new(),
        }
    }

    /// Add an allowed tool
    pub fn with_allowed_tool(mut self, tool: String) -> Self {
        self.allowed_tools.insert(tool);
        self
    }

    /// Add multiple allowed tools
    pub fn with_allowed_tools<I>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        for tool in tools {
            self.allowed_tools.insert(tool);
        }
        self
    }
}

impl Default for ToolAccess {
    fn default() -> Self {
        Self::new()
    }
}

/// Task complexity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskComplexity {
    Trivial,
    Simple,
    Medium,
    Complex,
    Expert,
}

/// Agent mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMapping {
    pub maestro_role: AgentRole,
    pub pi_agent_type: PiAgentType,
    pub tool_access: ToolAccess,
    pub complexity_range: (TaskComplexity, TaskComplexity),
    pub description: String,
}

impl AgentMapping {
    /// Create a new agent mapping
    pub fn new(
        maestro_role: AgentRole,
        pi_agent_type: PiAgentType,
        tool_access: ToolAccess,
        complexity_range: (TaskComplexity, TaskComplexity),
        description: String,
    ) -> Self {
        Self {
            maestro_role,
            pi_agent_type,
            tool_access,
            complexity_range,
            description,
        }
    }

    /// Check if this mapping can handle a given complexity
    pub fn can_handle_complexity(&self, complexity: TaskComplexity) -> bool {
        complexity >= self.complexity_range.0 && complexity <= self.complexity_range.1
    }
}

/// Default agent mappings
pub fn default_mappings() -> Vec<AgentMapping> {
    vec![
        AgentMapping {
            maestro_role: AgentRole::Scout,
            pi_agent_type: PiAgentType::Scout,
            tool_access: ToolAccess {
                can_read: true,
                can_write: false,
                can_execute: false,
                can_search: true,
                allowed_tools: {
                    let mut set = HashSet::new();
                    set.insert("grep".to_string());
                    set.insert("find".to_string());
                    set
                },
            },
            complexity_range: (TaskComplexity::Trivial, TaskComplexity::Simple),
            description: "Fast reconnaissance and information gathering".to_string(),
        },
        AgentMapping {
            maestro_role: AgentRole::Architect,
            pi_agent_type: PiAgentType::Planner,
            tool_access: ToolAccess {
                can_read: true,
                can_write: false,
                can_execute: false,
                can_search: true,
                allowed_tools: {
                    let mut set = HashSet::new();
                    set.insert("grep".to_string());
                    set.insert("find".to_string());
                    set.insert("read".to_string());
                    set
                },
            },
            complexity_range: (TaskComplexity::Simple, TaskComplexity::Complex),
            description: "Architecture design and planning".to_string(),
        },
        AgentMapping {
            maestro_role: AgentRole::Critic,
            pi_agent_type: PiAgentType::Reviewer,
            tool_access: ToolAccess {
                can_read: true,
                can_write: false,
                can_execute: false,
                can_search: true,
                allowed_tools: {
                    let mut set = HashSet::new();
                    set.insert("grep".to_string());
                    set.insert("read".to_string());
                    set
                },
            },
            complexity_range: (TaskComplexity::Simple, TaskComplexity::Complex),
            description: "Code review and quality analysis".to_string(),
        },
        AgentMapping {
            maestro_role: AgentRole::Kraken,
            pi_agent_type: PiAgentType::Worker,
            tool_access: ToolAccess {
                can_read: true,
                can_write: true,
                can_execute: true,
                can_search: true,
                allowed_tools: {
                    let mut set = HashSet::new();
                    set.insert("grep".to_string());
                    set.insert("find".to_string());
                    set.insert("read".to_string());
                    set.insert("write".to_string());
                    set.insert("execute".to_string());
                    set
                },
            },
            complexity_range: (TaskComplexity::Medium, TaskComplexity::Expert),
            description: "Implementation and execution".to_string(),
        },
    ]
}

/// Convert Maestro role to Pi-mono agent type
pub fn role_to_pi_agent_type(role: &AgentRole) -> Option<PiAgentType> {
    default_mappings()
        .iter()
        .find(|m| &m.maestro_role == role)
        .map(|m| m.pi_agent_type.clone())
}

/// Registered agent with all execution information
#[derive(Debug, Clone)]
pub struct RegisteredAgent {
    pub role: AgentRole,
    pub pi_agent_type: PiAgentType,
    pub model_id: String,
    pub provider: String,
    pub tool_access: ToolAccess,
    pub complexity_range: (TaskComplexity, TaskComplexity),
}

/// Agent registry for looking up and validating agents
pub struct AgentRegistry {
    config: Arc<PiMonoConfig>,
    detection: Option<PiDetection>,
    agent_cache: HashMap<AgentRole, RegisteredAgent>,
}

impl AgentRegistry {
    /// Create a new agent registry from configuration
    pub fn new(config: PiMonoConfig) -> Self {
        let mut registry = Self {
            config: Arc::new(config),
            detection: None,
            agent_cache: HashMap::new(),
        };
        // Build cache during construction - ignore errors as config may be incomplete
        let _ = registry.build_cache();
        registry
    }

    /// Set pi-mono detection info
    pub fn with_detection(mut self, detection: PiDetection) -> Self {
        self.detection = Some(detection);
        self
    }

    /// Look up an agent by role
    pub fn get_agent(&self, role: AgentRole) -> Result<RegisteredAgent> {
        self.agent_cache
            .get(&role)
            .cloned()
            .ok_or_else(|| Error::Other(format!("Agent not found for role: {:?}", role)))
    }

    /// Get pi-agent type for a role
    pub fn get_pi_agent_type(&self, role: AgentRole) -> Result<PiAgentType> {
        let agent = self.get_agent(role)?;
        Ok(agent.pi_agent_type)
    }

    /// Get model assignment for a role
    pub fn get_model_for_role(&self, role: AgentRole) -> Result<String> {
        let role_key = role_to_config_key(&role);
        self.config
            .role_assignments
            .get(&role_key)
            .map(|assignment| assignment.model_id.clone())
            .ok_or_else(|| Error::Other(format!("No model assignment found for role: {:?}", role)))
    }

    /// Validate tool access for a role
    pub fn validate_tool_access(&self, role: AgentRole, tool: &str) -> Result<bool> {
        // Early return for empty tool names
        if tool.is_empty() {
            return Ok(false);
        }

        let agent = self.get_agent(role)?;

        // Check if tool is in allowed_tools list
        if !agent.tool_access.allowed_tools.is_empty() {
            return Ok(agent.tool_access.allowed_tools.contains(tool));
        }

        // Fall back to permission-based checks
        match tool {
            t if t.contains("read") || t.contains("grep") || t.contains("find") => {
                Ok(agent.tool_access.can_read)
            }
            t if t.contains("write") || t.contains("edit") => Ok(agent.tool_access.can_write),
            t if t.contains("exec") || t.contains("run") => Ok(agent.tool_access.can_execute),
            t if t.contains("search") => Ok(agent.tool_access.can_search),
            _ => Ok(false),
        }
    }

    /// Check if a role can handle a task complexity
    pub fn can_handle_complexity(
        &self,
        role: AgentRole,
        complexity: TaskComplexity,
    ) -> Result<bool> {
        let agent = self.get_agent(role)?;
        Ok(complexity >= agent.complexity_range.0 && complexity <= agent.complexity_range.1)
    }

    /// Get all registered roles
    pub fn registered_roles(&self) -> Vec<AgentRole> {
        self.agent_cache.keys().cloned().collect()
    }

    /// Build the agent cache from config
    fn build_cache(&mut self) -> Result<()> {
        let mappings = default_mappings();

        for mapping in mappings {
            let role_key = role_to_config_key(&mapping.maestro_role);

            // Get model assignment from config or use a default
            let (model_id, provider) =
                if let Some(assignment) = self.config.role_assignments.get(&role_key) {
                    (assignment.model_id.clone(), assignment.provider.clone())
                } else {
                    // Default fallback when no assignment exists
                    ("claude-haiku-4-5".to_string(), "anthropic".to_string())
                };

            let registered = RegisteredAgent {
                role: mapping.maestro_role.clone(),
                pi_agent_type: mapping.pi_agent_type,
                model_id,
                provider,
                tool_access: mapping.tool_access,
                complexity_range: mapping.complexity_range,
            };

            self.agent_cache.insert(mapping.maestro_role, registered);
        }

        Ok(())
    }
}

/// Convert AgentRole to config key (lowercase string)
fn role_to_config_key(role: &AgentRole) -> String {
    match role {
        AgentRole::Scout => "scout".to_string(),
        AgentRole::Architect => "architect".to_string(),
        AgentRole::Critic => "critic".to_string(),
        AgentRole::Kraken => "kraken".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // AgentRole enum tests
    #[test]
    fn test_agent_role_variants() {
        let scout = AgentRole::Scout;
        let architect = AgentRole::Architect;
        let critic = AgentRole::Critic;
        let kraken = AgentRole::Kraken;

        assert_eq!(scout, AgentRole::Scout);
        assert_eq!(architect, AgentRole::Architect);
        assert_eq!(critic, AgentRole::Critic);
        assert_eq!(kraken, AgentRole::Kraken);
    }

    #[test]
    fn test_agent_role_equality() {
        assert_eq!(AgentRole::Scout, AgentRole::Scout);
        assert_ne!(AgentRole::Scout, AgentRole::Architect);
    }

    #[test]
    fn test_agent_role_hashable() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(AgentRole::Scout);
        set.insert(AgentRole::Architect);
        assert_eq!(set.len(), 2);
    }

    // PiAgentType enum tests
    #[test]
    fn test_pi_agent_type_variants() {
        let scout = PiAgentType::Scout;
        let planner = PiAgentType::Planner;
        let reviewer = PiAgentType::Reviewer;
        let worker = PiAgentType::Worker;

        assert_eq!(scout, PiAgentType::Scout);
        assert_eq!(planner, PiAgentType::Planner);
        assert_eq!(reviewer, PiAgentType::Reviewer);
        assert_eq!(worker, PiAgentType::Worker);
    }

    #[test]
    fn test_pi_agent_type_equality() {
        assert_eq!(PiAgentType::Scout, PiAgentType::Scout);
        assert_ne!(PiAgentType::Scout, PiAgentType::Planner);
    }

    #[test]
    fn test_pi_agent_type_hashable() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PiAgentType::Scout);
        set.insert(PiAgentType::Planner);
        assert_eq!(set.len(), 2);
    }

    // ToolAccess struct tests
    #[test]
    fn test_tool_access_new() {
        let access = ToolAccess::new();
        assert!(!access.can_read);
        assert!(!access.can_write);
        assert!(!access.can_execute);
        assert!(!access.can_search);
        assert!(access.allowed_tools.is_empty());
    }

    #[test]
    fn test_tool_access_read_only() {
        let access = ToolAccess::read_only();
        assert!(access.can_read);
        assert!(!access.can_write);
        assert!(!access.can_execute);
        assert!(access.can_search);
        assert!(access.allowed_tools.is_empty());
    }

    #[test]
    fn test_tool_access_full_access() {
        let access = ToolAccess::full_access();
        assert!(access.can_read);
        assert!(access.can_write);
        assert!(access.can_execute);
        assert!(access.can_search);
        assert!(access.allowed_tools.is_empty());
    }

    #[test]
    fn test_tool_access_with_allowed_tool() {
        let access = ToolAccess::new()
            .with_allowed_tool("grep".to_string())
            .with_allowed_tool("find".to_string());

        assert!(access.allowed_tools.contains("grep"));
        assert!(access.allowed_tools.contains("find"));
        assert_eq!(access.allowed_tools.len(), 2);
    }

    #[test]
    fn test_tool_access_with_allowed_tools() {
        let tools = vec!["grep".to_string(), "find".to_string(), "read".to_string()];
        let access = ToolAccess::new().with_allowed_tools(tools);

        assert!(access.allowed_tools.contains("grep"));
        assert!(access.allowed_tools.contains("find"));
        assert!(access.allowed_tools.contains("read"));
        assert_eq!(access.allowed_tools.len(), 3);
    }

    #[test]
    fn test_tool_access_default() {
        let access = ToolAccess::default();
        assert!(!access.can_read);
        assert!(!access.can_write);
        assert!(!access.can_execute);
        assert!(!access.can_search);
        assert!(access.allowed_tools.is_empty());
    }

    #[test]
    fn test_tool_access_equality() {
        let access1 = ToolAccess::new().with_allowed_tool("grep".to_string());
        let access2 = ToolAccess::new().with_allowed_tool("grep".to_string());
        assert_eq!(access1, access2);
    }

    // TaskComplexity enum tests
    #[test]
    fn test_task_complexity_variants() {
        let trivial = TaskComplexity::Trivial;
        let simple = TaskComplexity::Simple;
        let medium = TaskComplexity::Medium;
        let complex = TaskComplexity::Complex;
        let expert = TaskComplexity::Expert;

        assert_eq!(trivial, TaskComplexity::Trivial);
        assert_eq!(simple, TaskComplexity::Simple);
        assert_eq!(medium, TaskComplexity::Medium);
        assert_eq!(complex, TaskComplexity::Complex);
        assert_eq!(expert, TaskComplexity::Expert);
    }

    #[test]
    fn test_task_complexity_ordering() {
        assert!(TaskComplexity::Trivial < TaskComplexity::Simple);
        assert!(TaskComplexity::Simple < TaskComplexity::Medium);
        assert!(TaskComplexity::Medium < TaskComplexity::Complex);
        assert!(TaskComplexity::Complex < TaskComplexity::Expert);
    }

    #[test]
    fn test_task_complexity_total_ordering() {
        let mut complexities = vec![
            TaskComplexity::Expert,
            TaskComplexity::Trivial,
            TaskComplexity::Medium,
        ];
        complexities.sort();
        assert_eq!(
            complexities,
            vec![
                TaskComplexity::Trivial,
                TaskComplexity::Medium,
                TaskComplexity::Expert
            ]
        );
    }

    // AgentMapping tests
    #[test]
    fn test_agent_mapping_creation() {
        let mapping = AgentMapping::new(
            AgentRole::Scout,
            PiAgentType::Scout,
            ToolAccess::read_only(),
            (TaskComplexity::Trivial, TaskComplexity::Simple),
            "Fast reconnaissance".to_string(),
        );

        assert_eq!(mapping.maestro_role, AgentRole::Scout);
        assert_eq!(mapping.pi_agent_type, PiAgentType::Scout);
        assert!(mapping.tool_access.can_read);
        assert_eq!(mapping.complexity_range.0, TaskComplexity::Trivial);
        assert_eq!(mapping.complexity_range.1, TaskComplexity::Simple);
        assert_eq!(mapping.description, "Fast reconnaissance");
    }

    #[test]
    fn test_agent_mapping_can_handle_complexity_within_range() {
        let mapping = AgentMapping::new(
            AgentRole::Scout,
            PiAgentType::Scout,
            ToolAccess::read_only(),
            (TaskComplexity::Simple, TaskComplexity::Complex),
            "Test mapping".to_string(),
        );

        assert!(mapping.can_handle_complexity(TaskComplexity::Simple));
        assert!(mapping.can_handle_complexity(TaskComplexity::Medium));
        assert!(mapping.can_handle_complexity(TaskComplexity::Complex));
    }

    #[test]
    fn test_agent_mapping_can_handle_complexity_below_range() {
        let mapping = AgentMapping::new(
            AgentRole::Scout,
            PiAgentType::Scout,
            ToolAccess::read_only(),
            (TaskComplexity::Simple, TaskComplexity::Complex),
            "Test mapping".to_string(),
        );

        assert!(!mapping.can_handle_complexity(TaskComplexity::Trivial));
    }

    #[test]
    fn test_agent_mapping_can_handle_complexity_above_range() {
        let mapping = AgentMapping::new(
            AgentRole::Scout,
            PiAgentType::Scout,
            ToolAccess::read_only(),
            (TaskComplexity::Simple, TaskComplexity::Complex),
            "Test mapping".to_string(),
        );

        assert!(!mapping.can_handle_complexity(TaskComplexity::Expert));
    }

    #[test]
    fn test_agent_mapping_can_handle_complexity_exact_bounds() {
        let mapping = AgentMapping::new(
            AgentRole::Scout,
            PiAgentType::Scout,
            ToolAccess::read_only(),
            (TaskComplexity::Medium, TaskComplexity::Medium),
            "Test mapping".to_string(),
        );

        assert!(mapping.can_handle_complexity(TaskComplexity::Medium));
        assert!(!mapping.can_handle_complexity(TaskComplexity::Simple));
        assert!(!mapping.can_handle_complexity(TaskComplexity::Complex));
    }

    #[test]
    fn test_agent_mapping_serialization() {
        let mapping = AgentMapping::new(
            AgentRole::Scout,
            PiAgentType::Scout,
            ToolAccess::read_only(),
            (TaskComplexity::Trivial, TaskComplexity::Simple),
            "Fast reconnaissance".to_string(),
        );

        let serialized = serde_json::to_string(&mapping).unwrap();
        let deserialized: AgentMapping = serde_json::from_str(&serialized).unwrap();

        assert_eq!(mapping.maestro_role, deserialized.maestro_role);
        assert_eq!(mapping.pi_agent_type, deserialized.pi_agent_type);
        assert_eq!(mapping.description, deserialized.description);
    }

    // Default mappings tests
    #[test]
    fn test_default_mappings_count() {
        assert_eq!(default_mappings().len(), 4);
    }

    #[test]
    fn test_default_scout_mapping() {
        let mappings = default_mappings();
        let scout_mapping = mappings
            .iter()
            .find(|m| m.maestro_role == AgentRole::Scout)
            .expect("Scout mapping should exist");

        assert_eq!(scout_mapping.pi_agent_type, PiAgentType::Scout);
        assert!(scout_mapping.tool_access.can_read);
        assert!(!scout_mapping.tool_access.can_write);
        assert!(!scout_mapping.tool_access.can_execute);
        assert!(scout_mapping.tool_access.can_search);
        assert_eq!(
            scout_mapping.complexity_range,
            (TaskComplexity::Trivial, TaskComplexity::Simple)
        );
        assert!(scout_mapping.tool_access.allowed_tools.contains("grep"));
        assert!(scout_mapping.tool_access.allowed_tools.contains("find"));
    }

    #[test]
    fn test_default_architect_mapping() {
        let mappings = default_mappings();
        let architect_mapping = mappings
            .iter()
            .find(|m| m.maestro_role == AgentRole::Architect)
            .expect("Architect mapping should exist");

        assert_eq!(architect_mapping.pi_agent_type, PiAgentType::Planner);
        assert!(architect_mapping.tool_access.can_read);
        assert!(!architect_mapping.tool_access.can_write);
        assert!(!architect_mapping.tool_access.can_execute);
        assert!(architect_mapping.tool_access.can_search);
        assert_eq!(
            architect_mapping.complexity_range,
            (TaskComplexity::Simple, TaskComplexity::Complex)
        );
    }

    #[test]
    fn test_default_critic_mapping() {
        let mappings = default_mappings();
        let critic_mapping = mappings
            .iter()
            .find(|m| m.maestro_role == AgentRole::Critic)
            .expect("Critic mapping should exist");

        assert_eq!(critic_mapping.pi_agent_type, PiAgentType::Reviewer);
        assert!(critic_mapping.tool_access.can_read);
        assert!(!critic_mapping.tool_access.can_write);
        assert!(!critic_mapping.tool_access.can_execute);
        assert!(critic_mapping.tool_access.can_search);
        assert_eq!(
            critic_mapping.complexity_range,
            (TaskComplexity::Simple, TaskComplexity::Complex)
        );
    }

    #[test]
    fn test_default_kraken_mapping() {
        let mappings = default_mappings();
        let kraken_mapping = mappings
            .iter()
            .find(|m| m.maestro_role == AgentRole::Kraken)
            .expect("Kraken mapping should exist");

        assert_eq!(kraken_mapping.pi_agent_type, PiAgentType::Worker);
        assert!(kraken_mapping.tool_access.can_read);
        assert!(kraken_mapping.tool_access.can_write);
        assert!(kraken_mapping.tool_access.can_execute);
        assert!(kraken_mapping.tool_access.can_search);
        assert_eq!(
            kraken_mapping.complexity_range,
            (TaskComplexity::Medium, TaskComplexity::Expert)
        );
    }

    // Role to pi-agent mapping tests
    #[test]
    fn test_role_to_pi_agent_type_scout() {
        let pi_type = role_to_pi_agent_type(&AgentRole::Scout);
        assert_eq!(pi_type, Some(PiAgentType::Scout));
    }

    #[test]
    fn test_role_to_pi_agent_type_architect() {
        let pi_type = role_to_pi_agent_type(&AgentRole::Architect);
        assert_eq!(pi_type, Some(PiAgentType::Planner));
    }

    #[test]
    fn test_role_to_pi_agent_type_critic() {
        let pi_type = role_to_pi_agent_type(&AgentRole::Critic);
        assert_eq!(pi_type, Some(PiAgentType::Reviewer));
    }

    #[test]
    fn test_role_to_pi_agent_type_kraken() {
        let pi_type = role_to_pi_agent_type(&AgentRole::Kraken);
        assert_eq!(pi_type, Some(PiAgentType::Worker));
    }

    // Integration test
    #[test]
    fn test_complete_maestro_to_pi_mapping() {
        let mappings = vec![
            (AgentRole::Scout, PiAgentType::Scout),
            (AgentRole::Architect, PiAgentType::Planner),
            (AgentRole::Critic, PiAgentType::Reviewer),
            (AgentRole::Kraken, PiAgentType::Worker),
        ];

        for (role, expected_pi_type) in mappings {
            let pi_type = role_to_pi_agent_type(&role);
            assert_eq!(
                pi_type,
                Some(expected_pi_type.clone()),
                "Role {:?} should map to {:?}",
                role,
                expected_pi_type
            );
        }
    }

    // AgentRegistry tests
    mod agent_registry_tests {
        use super::*;
        use crate::config::models::{PiMonoConfig, RoleAssignment};
        use std::collections::HashMap;

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
            role_assignments.insert(
                "architect".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: Some(true),
                },
            );
            role_assignments.insert(
                "critic".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );
            role_assignments.insert(
                "kraken".to_string(),
                RoleAssignment {
                    model_id: "claude-opus-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: Some(vec!["claude-sonnet-4-5".to_string()]),
                    use_reasoning: Some(true),
                },
            );

            PiMonoConfig {
                role_assignments,
                ..Default::default()
            }
        }

        #[test]
        fn test_agent_registry_new() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            // Verify registry was created
            assert_eq!(registry.registered_roles().len(), 4);
        }

        #[test]
        fn test_agent_registry_new_empty_config() {
            let config = PiMonoConfig::default();
            let registry = AgentRegistry::new(config);

            // Registry should still cache all 4 default agents
            assert_eq!(registry.registered_roles().len(), 4);
        }

        #[test]
        fn test_get_agent_scout() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            let agent = registry.get_agent(AgentRole::Scout).unwrap();
            assert_eq!(agent.role, AgentRole::Scout);
            assert_eq!(agent.pi_agent_type, PiAgentType::Scout);
            assert_eq!(agent.model_id, "claude-haiku-4-5");
            assert_eq!(agent.provider, "anthropic");
            assert!(agent.tool_access.can_read);
            assert!(!agent.tool_access.can_write);
            assert!(!agent.tool_access.can_execute);
            assert!(agent.tool_access.can_search);
        }

        #[test]
        fn test_get_agent_architect() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            let agent = registry.get_agent(AgentRole::Architect).unwrap();
            assert_eq!(agent.role, AgentRole::Architect);
            assert_eq!(agent.pi_agent_type, PiAgentType::Planner);
            assert_eq!(agent.model_id, "claude-sonnet-4-5");
            assert_eq!(agent.provider, "anthropic");
        }

        #[test]
        fn test_get_agent_critic() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            let agent = registry.get_agent(AgentRole::Critic).unwrap();
            assert_eq!(agent.role, AgentRole::Critic);
            assert_eq!(agent.pi_agent_type, PiAgentType::Reviewer);
            assert_eq!(agent.model_id, "claude-sonnet-4-5");
        }

        #[test]
        fn test_get_agent_kraken() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            let agent = registry.get_agent(AgentRole::Kraken).unwrap();
            assert_eq!(agent.role, AgentRole::Kraken);
            assert_eq!(agent.pi_agent_type, PiAgentType::Worker);
            assert_eq!(agent.model_id, "claude-opus-4-5");
            assert!(agent.tool_access.can_read);
            assert!(agent.tool_access.can_write);
            assert!(agent.tool_access.can_execute);
            assert!(agent.tool_access.can_search);
        }

        #[test]
        fn test_get_pi_agent_type() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            assert_eq!(
                registry.get_pi_agent_type(AgentRole::Scout).unwrap(),
                PiAgentType::Scout
            );
            assert_eq!(
                registry.get_pi_agent_type(AgentRole::Architect).unwrap(),
                PiAgentType::Planner
            );
            assert_eq!(
                registry.get_pi_agent_type(AgentRole::Critic).unwrap(),
                PiAgentType::Reviewer
            );
            assert_eq!(
                registry.get_pi_agent_type(AgentRole::Kraken).unwrap(),
                PiAgentType::Worker
            );
        }

        #[test]
        fn test_get_model_for_role() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            assert_eq!(
                registry.get_model_for_role(AgentRole::Scout).unwrap(),
                "claude-haiku-4-5"
            );
            assert_eq!(
                registry.get_model_for_role(AgentRole::Architect).unwrap(),
                "claude-sonnet-4-5"
            );
            assert_eq!(
                registry.get_model_for_role(AgentRole::Critic).unwrap(),
                "claude-sonnet-4-5"
            );
            assert_eq!(
                registry.get_model_for_role(AgentRole::Kraken).unwrap(),
                "claude-opus-4-5"
            );
        }

        #[test]
        fn test_get_model_for_role_missing_assignment() {
            let config = PiMonoConfig::default();
            let registry = AgentRegistry::new(config);

            // Should return error when no assignment exists
            let result = registry.get_model_for_role(AgentRole::Scout);
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("No model assignment found"));
        }

        #[test]
        fn test_validate_tool_access_scout() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            // Scout can use tools in allowed_tools list (grep, find from default mappings)
            assert!(registry
                .validate_tool_access(AgentRole::Scout, "grep")
                .unwrap());
            assert!(registry
                .validate_tool_access(AgentRole::Scout, "find")
                .unwrap());

            // Scout cannot use tools not in allowed_tools list
            assert!(!registry
                .validate_tool_access(AgentRole::Scout, "read-file")
                .unwrap());
            assert!(!registry
                .validate_tool_access(AgentRole::Scout, "search")
                .unwrap());
            assert!(!registry
                .validate_tool_access(AgentRole::Scout, "write-file")
                .unwrap());
            assert!(!registry
                .validate_tool_access(AgentRole::Scout, "execute")
                .unwrap());
            assert!(!registry
                .validate_tool_access(AgentRole::Scout, "run-command")
                .unwrap());
        }

        #[test]
        fn test_validate_tool_access_kraken() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            // Kraken can use tools in allowed_tools list (grep, find, read, write, execute from default mappings)
            assert!(registry
                .validate_tool_access(AgentRole::Kraken, "grep")
                .unwrap());
            assert!(registry
                .validate_tool_access(AgentRole::Kraken, "find")
                .unwrap());
            assert!(registry
                .validate_tool_access(AgentRole::Kraken, "read")
                .unwrap());
            assert!(registry
                .validate_tool_access(AgentRole::Kraken, "write")
                .unwrap());
            assert!(registry
                .validate_tool_access(AgentRole::Kraken, "execute")
                .unwrap());
        }

        #[test]
        fn test_validate_tool_access_with_allowed_tools() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            // Scout has specific allowed tools in the mapping
            assert!(registry
                .validate_tool_access(AgentRole::Scout, "grep")
                .unwrap());
            assert!(registry
                .validate_tool_access(AgentRole::Scout, "find")
                .unwrap());

            // Tool not in allowed_tools list
            assert!(!registry
                .validate_tool_access(AgentRole::Scout, "custom-tool")
                .unwrap());
        }

        #[test]
        fn test_validate_tool_access_empty_string() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            // Empty tool name should return false
            assert!(!registry.validate_tool_access(AgentRole::Scout, "").unwrap());
            assert!(!registry
                .validate_tool_access(AgentRole::Kraken, "")
                .unwrap());
            assert!(!registry
                .validate_tool_access(AgentRole::Architect, "")
                .unwrap());
            assert!(!registry
                .validate_tool_access(AgentRole::Critic, "")
                .unwrap());
        }

        #[test]
        fn test_can_handle_complexity_scout() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            // Scout: Trivial to Simple
            assert!(registry
                .can_handle_complexity(AgentRole::Scout, TaskComplexity::Trivial)
                .unwrap());
            assert!(registry
                .can_handle_complexity(AgentRole::Scout, TaskComplexity::Simple)
                .unwrap());
            assert!(!registry
                .can_handle_complexity(AgentRole::Scout, TaskComplexity::Medium)
                .unwrap());
            assert!(!registry
                .can_handle_complexity(AgentRole::Scout, TaskComplexity::Complex)
                .unwrap());
            assert!(!registry
                .can_handle_complexity(AgentRole::Scout, TaskComplexity::Expert)
                .unwrap());
        }

        #[test]
        fn test_can_handle_complexity_architect() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            // Architect: Simple to Complex
            assert!(!registry
                .can_handle_complexity(AgentRole::Architect, TaskComplexity::Trivial)
                .unwrap());
            assert!(registry
                .can_handle_complexity(AgentRole::Architect, TaskComplexity::Simple)
                .unwrap());
            assert!(registry
                .can_handle_complexity(AgentRole::Architect, TaskComplexity::Medium)
                .unwrap());
            assert!(registry
                .can_handle_complexity(AgentRole::Architect, TaskComplexity::Complex)
                .unwrap());
            assert!(!registry
                .can_handle_complexity(AgentRole::Architect, TaskComplexity::Expert)
                .unwrap());
        }

        #[test]
        fn test_can_handle_complexity_kraken() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            // Kraken: Medium to Expert
            assert!(!registry
                .can_handle_complexity(AgentRole::Kraken, TaskComplexity::Trivial)
                .unwrap());
            assert!(!registry
                .can_handle_complexity(AgentRole::Kraken, TaskComplexity::Simple)
                .unwrap());
            assert!(registry
                .can_handle_complexity(AgentRole::Kraken, TaskComplexity::Medium)
                .unwrap());
            assert!(registry
                .can_handle_complexity(AgentRole::Kraken, TaskComplexity::Complex)
                .unwrap());
            assert!(registry
                .can_handle_complexity(AgentRole::Kraken, TaskComplexity::Expert)
                .unwrap());
        }

        #[test]
        fn test_registered_roles() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            let roles = registry.registered_roles();
            assert_eq!(roles.len(), 4);
            assert!(roles.contains(&AgentRole::Scout));
            assert!(roles.contains(&AgentRole::Architect));
            assert!(roles.contains(&AgentRole::Critic));
            assert!(roles.contains(&AgentRole::Kraken));
        }

        #[test]
        fn test_with_detection() {
            let config = create_test_config();
            let detection = PiDetection {
                executable_path: std::path::PathBuf::from("/usr/local/bin/pi"),
                version: Some("0.49.3".to_string()),
                capabilities: crate::detection::Capabilities::default(),
            };

            let registry = AgentRegistry::new(config).with_detection(detection);

            // Verify registry still works
            assert_eq!(registry.registered_roles().len(), 4);
            let agent = registry.get_agent(AgentRole::Scout).unwrap();
            assert_eq!(agent.role, AgentRole::Scout);
        }

        #[test]
        fn test_role_to_config_key() {
            assert_eq!(role_to_config_key(&AgentRole::Scout), "scout");
            assert_eq!(role_to_config_key(&AgentRole::Architect), "architect");
            assert_eq!(role_to_config_key(&AgentRole::Critic), "critic");
            assert_eq!(role_to_config_key(&AgentRole::Kraken), "kraken");
        }

        #[test]
        fn test_registered_agent_clone() {
            let config = create_test_config();
            let registry = AgentRegistry::new(config);

            let agent1 = registry.get_agent(AgentRole::Scout).unwrap();
            let agent2 = agent1.clone();

            assert_eq!(agent1.role, agent2.role);
            assert_eq!(agent1.pi_agent_type, agent2.pi_agent_type);
            assert_eq!(agent1.model_id, agent2.model_id);
            assert_eq!(agent1.provider, agent2.provider);
        }
    }
}
