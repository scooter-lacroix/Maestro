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

pub mod agents;
pub mod config;
pub mod detection;
pub mod discovery;
pub mod error;
pub mod execution;

// Public re-exports for convenience
pub use config::{
    io::{
        config_dir, config_path, default_config, ensure_config_dir, load_config,
        load_config_from_path, save_config, save_config_to_path,
        validate_config as validate_config_basic,
    },
    validation::{
        validate_config_ext, validate_model_assignments, validate_pi_path, ValidationSeverity,
        ValidationWarning,
    },
    wizard::{ConfigWizard, WizardState, WizardStep},
    ExecutionSettings, ModelPreference, ModelSelector, ModelTier, PiMonoConfig, ProviderConfig,
    RoleAssignment,
};
// ModelConfig is the PiMonoConfig from the models module (with role_assignments)
pub use agents::mapping::{
    default_mappings, role_to_pi_agent_type, AgentMapping, AgentRegistry, AgentRole, PiAgentType,
    RegisteredAgent, TaskComplexity, ToolAccess,
};
pub use agents::workflows::{
    default_presets, get_preset, preset_names, WorkflowMode, WorkflowPreset, WorkflowStep,
};
pub use agents::{AgentError, PiMonoAgent};
pub use config::models::PiMonoConfig as ModelConfig;
pub use detection::{Capabilities, PiDetection};
pub use discovery::{
    DiscoveryResult, ModelDiscovery, ModelInfo, ProviderStatus, DEFAULT_CACHE_DURATION_SECS,
};
pub use execution::{
    runner::{ChainResult, ChainStep, ParallelResult, ParallelTask, RunnerConfig, SubagentRunner},
    ExecutionResult, Executor, ExecutorConfig, StreamEvent, StreamEventType, SubagentResult,
    UsageMetrics,
};

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
