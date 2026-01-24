//! # Agents module for Pi-Mono integration
//!
//! This module provides interfaces for interacting with Pi-Mono agents.
//!
//! ## Example
//!
//! ```rust
//! use maestro_pi_mono::agents::PiMonoAgent;
//!
//! // Create a new agent
//! let agent = PiMonoAgent::new(
//!     "agent-001".to_string(),
//!     "My Agent".to_string()
//! ).unwrap();
//!
//! // Add a description using the builder pattern
//! let agent = agent.with_description("An example agent".to_string());
//! ```

pub mod mapping;
pub mod workflows;

use serde::{Deserialize, Serialize};

/// Error type for agent validation
#[derive(Debug, Clone, thiserror::Error)]
pub enum AgentError {
    #[error("Agent ID cannot be empty")]
    EmptyId,
    #[error("Agent name cannot be empty")]
    EmptyName,
}

/// Represents a Pi-Mono agent.
///
/// # Examples
///
/// Creating a basic agent:
///
/// ```rust
/// use maestro_pi_mono::agents::PiMonoAgent;
///
/// let agent = PiMonoAgent::new(
///     "agent-001".to_string(),
///     "My Agent".to_string()
/// ).unwrap();
/// assert_eq!(agent.id, "agent-001");
/// assert_eq!(agent.name, "My Agent");
/// assert!(agent.description.is_none());
/// ```
///
/// Creating an agent with a description:
///
/// ```rust
/// use maestro_pi_mono::agents::PiMonoAgent;
///
/// let agent = PiMonoAgent::new(
///     "agent-002".to_string(),
///     "Data Processor".to_string()
/// ).unwrap().with_description("Processes data from external sources".to_string());
///
/// assert_eq!(agent.description, Some("Processes data from external sources".to_string()));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiMonoAgent {
    /// Agent identifier
    pub id: String,

    /// Agent name
    pub name: String,

    /// Agent description
    pub description: Option<String>,
}

impl PiMonoAgent {
    /// Create a new Pi-Mono agent.
    ///
    /// # Arguments
    ///
    /// * `id` - A unique identifier for the agent
    /// * `name` - A human-readable name for the agent
    ///
    /// # Errors
    ///
    /// Returns `AgentError::EmptyId` if the id is an empty string.
    /// Returns `AgentError::EmptyName` if the name is an empty string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::agents::PiMonoAgent;
    ///
    /// let agent = PiMonoAgent::new(
    ///     "agent-001".to_string(),
    ///     "My Agent".to_string()
    /// ).unwrap();
    /// ```
    pub fn new(id: String, name: String) -> Result<Self, AgentError> {
        if id.trim().is_empty() {
            return Err(AgentError::EmptyId);
        }
        if name.trim().is_empty() {
            return Err(AgentError::EmptyName);
        }
        Ok(Self {
            id,
            name,
            description: None,
        })
    }

    /// Set the agent description.
    ///
    /// This method uses the builder pattern to allow chaining.
    ///
    /// # Arguments
    ///
    /// * `description` - A description of what the agent does
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::agents::PiMonoAgent;
    ///
    /// let agent = PiMonoAgent::new(
    ///     "agent-001".to_string(),
    ///     "My Agent".to_string()
    /// ).unwrap().with_description("An example agent".to_string());
    ///
    /// assert_eq!(agent.description, Some("An example agent".to_string()));
    /// ```
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation_valid() {
        let agent = PiMonoAgent::new("agent-001".to_string(), "My Agent".to_string()).unwrap();
        assert_eq!(agent.id, "agent-001");
        assert_eq!(agent.name, "My Agent");
        assert!(agent.description.is_none());
    }

    #[test]
    fn test_agent_creation_empty_id() {
        let result = PiMonoAgent::new("".to_string(), "My Agent".to_string());
        assert!(matches!(result, Err(AgentError::EmptyId)));
    }

    #[test]
    fn test_agent_creation_whitespace_only_id() {
        let result = PiMonoAgent::new("   ".to_string(), "My Agent".to_string());
        assert!(matches!(result, Err(AgentError::EmptyId)));
    }

    #[test]
    fn test_agent_creation_empty_name() {
        let result = PiMonoAgent::new("agent-001".to_string(), "".to_string());
        assert!(matches!(result, Err(AgentError::EmptyName)));
    }

    #[test]
    fn test_agent_creation_whitespace_only_name() {
        let result = PiMonoAgent::new("agent-001".to_string(), "   ".to_string());
        assert!(matches!(result, Err(AgentError::EmptyName)));
    }

    #[test]
    fn test_agent_with_description() {
        let agent = PiMonoAgent::new("agent-001".to_string(), "My Agent".to_string())
            .unwrap()
            .with_description("An example agent".to_string());

        assert_eq!(agent.description, Some("An example agent".to_string()));
    }

    #[test]
    fn test_agent_serialization() {
        let agent = PiMonoAgent::new("agent-001".to_string(), "My Agent".to_string())
            .unwrap()
            .with_description("An example agent".to_string());

        let serialized = serde_json::to_string(&agent).unwrap();
        let deserialized: PiMonoAgent = serde_json::from_str(&serialized).unwrap();

        assert_eq!(agent.id, deserialized.id);
        assert_eq!(agent.name, deserialized.name);
        assert_eq!(agent.description, deserialized.description);
    }
}
