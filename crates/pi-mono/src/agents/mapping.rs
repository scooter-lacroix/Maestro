//! # Agent mapping structures for Maestro to Pi-Mono integration
//!
//! This module provides data structures for mapping Maestro agent roles to Pi-Mono agent types,
//! including tool access permissions and task complexity levels.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Maestro agent roles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentRole {
    Scout,
    Architect,
    Critic,
    Kraken,
}

/// Pi-mono agent types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
                }
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
                }
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
                }
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
                }
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
        assert!(scout_mapping
            .tool_access
            .allowed_tools
            .contains("grep"));
        assert!(scout_mapping
            .tool_access
            .allowed_tools
            .contains("find"));
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
                role, expected_pi_type
            );
        }
    }
}
