//! # Maestro Pi-Mono Integration
//!
//! This crate provides integration with Pi-Mono agents for the Maestro system.
//!
//! ## Overview
//!
//! The `maestro-pi-mono` crate enables Maestro to interact with Pi-Mono agents,
//! providing configuration management, agent interfaces, and execution capabilities.
//!
//! ## Modules
//!
//! - [`config`] - Configuration loading and management for Pi-Mono agents
//! - [`agents`] - Interfaces for interacting with Pi-Mono agents
//! - [`execution`] - Execution of Pi-Mono agents and tasks
//! - [`error`] - Error types for the crate
//!
//! ## Example
//!
//! ```rust
//! use maestro_pi_mono::{agents::PiMonoAgent, execution::Executor};
//!
//! // Create a new agent
//! let agent = PiMonoAgent::new(
//!     "agent-001".to_string(),
//!     "My Agent".to_string()
//! ).unwrap().with_description("An example agent".to_string());
//!
//! // Create an executor (default configuration)
//! let executor = Executor::default();
//! ```
//!
//! ## Version Information
//!
//! To get the version of the crate at runtime:
//!
//! ```rust
//! let version = maestro_pi_mono::version();
//! println!("Maestro Pi-Mono version: {}", version);
//! ```

pub mod config;
pub mod agents;
pub mod execution;
pub mod error;
pub mod detection;
pub mod discovery;

// Public re-exports for convenience
pub use config::{
    PiMonoConfig, ModelTier, ModelPreference, ProviderConfig, RoleAssignment, ExecutionSettings, ModelSelector,
    io::{config_dir, config_path, ensure_config_dir, load_config, load_config_from_path, save_config, save_config_to_path, default_config, validate_config},
    wizard::{WizardStep, WizardState, ConfigWizard},
};
// ModelConfig is the PiMonoConfig from the models module (with role_assignments)
pub use config::models::PiMonoConfig as ModelConfig;
pub use agents::{PiMonoAgent, AgentError};
pub use agents::mapping::{
    AgentRole, PiAgentType, ToolAccess, TaskComplexity, AgentMapping,
    RegisteredAgent, AgentRegistry,
    default_mappings, role_to_pi_agent_type,
};
pub use agents::workflows::{
    WorkflowMode, WorkflowStep, WorkflowPreset,
    default_presets, get_preset, preset_names,
};
pub use execution::{
    Executor, ExecutorConfig, ExecutionResult,
    UsageMetrics, StreamEvent, StreamEventType, SubagentResult,
    runner::{SubagentRunner, RunnerConfig, ParallelTask, ParallelResult, ChainStep, ChainResult},
};
pub use detection::{PiDetection, Capabilities};
pub use discovery::{ModelDiscovery, ModelInfo, ProviderStatus, DiscoveryResult, DEFAULT_CACHE_DURATION_SECS};

/// Returns the version of the maestro-pi-mono crate.
///
/// # Example
///
/// ```rust
/// let version = maestro_pi_mono::version();
/// assert!(!version.is_empty());
/// println!("Running maestro-pi-mono version: {}", version);
/// ```
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
