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
//! );
//!
//! // Add a description using the builder pattern
//! let agent = agent.with_description("An example agent".to_string());
//! ```

use serde::{Deserialize, Serialize};

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
/// );
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
/// ).with_description("Processes data from external sources".to_string());
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
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::agents::PiMonoAgent;
    ///
    /// let agent = PiMonoAgent::new(
    ///     "agent-001".to_string(),
    ///     "My Agent".to_string()
    /// );
    /// ```
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            description: None,
        }
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
    /// ).with_description("An example agent".to_string());
    ///
    /// assert_eq!(agent.description, Some("An example agent".to_string()));
    /// ```
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}
