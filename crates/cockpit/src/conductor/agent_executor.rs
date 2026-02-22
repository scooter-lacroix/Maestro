//! Agent Executor - Unified backend abstraction for Pi-Mono and OMP agents
//!
//! This module provides a unified interface for executing agent tasks with
//! automatic backend selection (Pi-Mono preferred, OMP fallback).
//!
//! ## Architecture
//!
//! ```text
//! ConductorPane
//!   └── AgentExecutor
//!         ├── PiMonoBackend (native Rust, preferred)
//!         └── OmpBackend (subprocess, fallback)
//! ```

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use super::omp_agent::{OmpAgentConfig, OmpAgentManager};
use maestro_pi_mono::{
    agents::mapping::{AgentRole, PiAgentType},
    execution::{StreamEvent as PiStreamEvent, StreamEventType, SubagentResult, UsageMetrics},
    AgentRegistry, ModelConfig, PiDetection, SubagentRunner,
};

/// Convert pi-mono error to anyhow error
fn convert_pi_error(e: maestro_pi_mono::error::Error) -> anyhow::Error {
    anyhow!("Pi-Mono error: {}", e)
}

/// Agent execution configuration
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Model to use for this agent
    pub model: String,
    /// Tools enabled for this agent
    pub tools: Vec<String>,
    /// Timeout for execution
    pub timeout_secs: u64,
    /// Agent role (for Pi-Mono)
    pub agent_role: Option<AgentRole>,
    /// Working directory
    pub working_dir: Option<PathBuf>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "claude-3-5-sonnet".to_string(),
            tools: vec![
                "python".to_string(),
                "edit".to_string(),
                "grep".to_string(),
                "find".to_string(),
                "read".to_string(),
                "write".to_string(),
            ],
            timeout_secs: 300,
            agent_role: None,
            working_dir: None,
        }
    }
}

impl From<&OmpAgentConfig> for AgentConfig {
    fn from(omp_config: &OmpAgentConfig) -> Self {
        Self {
            model: omp_config.model.clone(),
            tools: omp_config.tools.clone(),
            timeout_secs: omp_config.timeout_secs,
            agent_role: None,
            working_dir: None,
        }
    }
}

/// Result of agent execution
#[derive(Debug, Clone)]
pub struct AgentResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Output from execution
    pub output: String,
    /// Error message if failed
    pub error: Option<String>,
    /// Usage metrics (if available)
    pub usage: Option<UsageMetrics>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Backend that was used
    pub backend: BackendType,
}

impl AgentResult {
    /// Create a successful result
    pub fn success(output: String, backend: BackendType) -> Self {
        Self {
            success: true,
            output,
            error: None,
            usage: None,
            duration_ms: 0,
            backend,
        }
    }

    /// Create a failed result
    pub fn failure(error: String, backend: BackendType) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error),
            usage: None,
            duration_ms: 0,
            backend,
        }
    }
}

/// Stream events from agent execution
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Execution started
    Started,
    /// Output text chunk
    Output(String),
    /// Progress update
    Progress {
        message: String,
        percent: Option<u8>,
    },
    /// Error occurred
    Error(String),
    /// Execution completed
    Completed { success: bool },
}

impl From<PiStreamEvent> for StreamEvent {
    fn from(event: PiStreamEvent) -> Self {
        match event.event_type {
            StreamEventType::Start => StreamEvent::Started,
            StreamEventType::Data => StreamEvent::Output(event.content),
            StreamEventType::Progress => StreamEvent::Progress {
                message: event.content,
                percent: event.metadata.and_then(|m| m.parse().ok()),
            },
            StreamEventType::Error => StreamEvent::Error(event.content),
            StreamEventType::Complete => StreamEvent::Completed {
                success: !event.content.to_lowercase().contains("error"),
            },
        }
    }
}

/// Type of backend used for execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// Native Rust Pi-Mono backend
    PiMono,
    /// TypeScript/Bun OMP backend
    Omp,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::PiMono => write!(f, "Pi-Mono"),
            BackendType::Omp => write!(f, "OMP"),
        }
    }
}

/// Callback type for streaming events
pub type StreamCallback = Box<dyn Fn(StreamEvent) + Send + Sync>;

/// Unified backend trait for agent execution
#[async_trait]
pub trait AgentBackend: Send + Sync {
    /// Check if this backend is available
    fn is_available(&self) -> bool;

    /// Get the backend type
    fn backend_type(&self) -> BackendType;

    /// Execute a task without streaming
    async fn execute(
        &self,
        task: &str,
        config: &AgentConfig,
        cancel: Option<CancellationToken>,
    ) -> Result<AgentResult>;

    /// Execute a task with streaming callbacks
    async fn execute_with_streaming(
        &self,
        task: &str,
        config: &AgentConfig,
        cancel: Option<CancellationToken>,
        callback: StreamCallback,
    ) -> Result<AgentResult>;
}

/// Pi-Mono backend implementation
pub struct PiMonoBackend {
    runner: Arc<SubagentRunner>,
    registry: Arc<AgentRegistry>,
    detection: Option<PiDetection>,
}

impl PiMonoBackend {
    /// Create a new Pi-Mono backend
    pub fn new(config: Option<Arc<ModelConfig>>) -> Self {
        let detection = PiDetection::detect().ok();
        let pi_mono_config = config.unwrap_or_else(|| Arc::new(ModelConfig::default()));
        let registry = Arc::new(AgentRegistry::new((*pi_mono_config).clone()));
        let runner = Arc::new(SubagentRunner::new());

        Self {
            runner,
            registry,
            detection,
        }
    }

    /// Get the agent type for a role
    fn get_agent_type(&self, role: &AgentRole) -> PiAgentType {
        self.registry
            .get_pi_agent_type(role.clone())
            .unwrap_or(PiAgentType::Worker)
    }

    /// Convert SubagentResult to AgentResult
    fn convert_result(&self, result: SubagentResult) -> AgentResult {
        AgentResult {
            success: result.success,
            output: result.output,
            error: result.error,
            usage: result.usage,
            duration_ms: result.duration.as_millis() as u64,
            backend: BackendType::PiMono,
        }
    }
}

#[async_trait]
impl AgentBackend for PiMonoBackend {
    fn is_available(&self) -> bool {
        self.detection.is_some()
    }

    fn backend_type(&self) -> BackendType {
        BackendType::PiMono
    }

    async fn execute(
        &self,
        task: &str,
        config: &AgentConfig,
        cancel: Option<CancellationToken>,
    ) -> Result<AgentResult> {
        let agent_type = config
            .agent_role
            .as_ref()
            .map(|r| self.get_agent_type(r))
            .unwrap_or(PiAgentType::Worker);

        info!(
            "Executing task via Pi-Mono backend with agent type: {:?}",
            agent_type
        );

        // Use the prompt parameter for additional context from working_dir if set
        let prompt = config
            .working_dir
            .as_ref()
            .map(|p| format!("Working directory: {}", p.display()));
        let prompt_str = prompt.as_deref();

        let result = if let Some(ref token) = cancel {
            self.runner
                .run_with_token(agent_type, task, prompt_str, Some(token))
                .await
                .map_err(convert_pi_error)?
        } else {
            self.runner
                .run(agent_type, task, prompt_str)
                .await
                .map_err(convert_pi_error)?
        };

        Ok(self.convert_result(result))
    }

    async fn execute_with_streaming(
        &self,
        task: &str,
        config: &AgentConfig,
        cancel: Option<CancellationToken>,
        callback: StreamCallback,
    ) -> Result<AgentResult> {
        let agent_type = config
            .agent_role
            .as_ref()
            .map(|r| self.get_agent_type(r))
            .unwrap_or(PiAgentType::Worker);

        info!(
            "Executing task via Pi-Mono backend with streaming, agent type: {:?}",
            agent_type
        );

        // Use the prompt parameter for additional context from working_dir if set
        let prompt = config
            .working_dir
            .as_ref()
            .map(|p| format!("Working directory: {}", p.display()));
        let prompt_str = prompt.as_deref();

        callback(StreamEvent::Started);

        let cancel_ref = cancel.as_ref();
        let result = self
            .runner
            .run_with_stream_and_token(
                agent_type,
                task,
                prompt_str,
                |event| {
                    callback(StreamEvent::from(event));
                },
                cancel_ref,
            )
            .await
            .map_err(convert_pi_error)?;

        callback(StreamEvent::Completed {
            success: result.success,
        });

        Ok(self.convert_result(result))
    }
}

/// OMP backend implementation (wraps existing OmpAgentManager)
pub struct OmpBackend {
    manager: Arc<OmpAgentManager>,
    track_id: String,
    project_path: PathBuf,
}

impl OmpBackend {
    /// Create a new OMP backend
    pub fn new(manager: Arc<OmpAgentManager>, track_id: String, project_path: PathBuf) -> Self {
        Self {
            manager,
            track_id,
            project_path,
        }
    }
}

#[async_trait]
impl AgentBackend for OmpBackend {
    fn is_available(&self) -> bool {
        self.manager.is_available()
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Omp
    }

    async fn execute(
        &self,
        task: &str,
        config: &AgentConfig,
        _cancel: Option<CancellationToken>,
    ) -> Result<AgentResult> {
        debug!("Executing task via OMP backend: {}", task);

        let omp_config = OmpAgentConfig {
            model: config.model.clone(),
            tools: config.tools.clone(),
            timeout_secs: config.timeout_secs,
            use_shared_pools: true,
        };

        let agent = self
            .manager
            .get_or_create_agent(
                self.track_id.clone(),
                self.project_path.clone(),
                Some(omp_config),
            )
            .await?;

        match agent.execute_task(task).await {
            Ok(output) => Ok(AgentResult::success(output, BackendType::Omp)),
            Err(e) => Ok(AgentResult::failure(e.to_string(), BackendType::Omp)),
        }
    }

    async fn execute_with_streaming(
        &self,
        task: &str,
        config: &AgentConfig,
        cancel: Option<CancellationToken>,
        callback: StreamCallback,
    ) -> Result<AgentResult> {
        callback(StreamEvent::Started);

        // OMP doesn't support streaming, so we execute and send completed event
        let result = self.execute(task, config, cancel).await?;

        callback(StreamEvent::Output(result.output.clone()));
        callback(StreamEvent::Completed {
            success: result.success,
        });

        Ok(result)
    }
}

/// Combined executor with automatic backend selection
pub struct AgentExecutor {
    pi_mono_backend: Option<Arc<PiMonoBackend>>,
    omp_backend: Option<Arc<OmpBackend>>,
    active_backend: Arc<RwLock<Option<BackendType>>>,
    cancellation_token: Arc<RwLock<Option<Arc<CancellationToken>>>>,
}

impl AgentExecutor {
    /// Create a new AgentExecutor
    pub fn new(
        pi_mono_config: Option<Arc<ModelConfig>>,
        omp_manager: Option<&OmpAgentManager>,
        track_id: Option<String>,
        project_path: Option<PathBuf>,
    ) -> Self {
        // Try to create Pi-Mono backend
        let pi_mono_backend = if pi_mono_config.is_some() {
            let backend = Arc::new(PiMonoBackend::new(pi_mono_config));
            if backend.is_available() {
                info!("Pi-Mono backend available");
                Some(backend)
            } else {
                warn!("Pi-Mono config provided but backend not available");
                None
            }
        } else {
            let backend = Arc::new(PiMonoBackend::new(None));
            if backend.is_available() {
                info!("Pi-Mono backend available (default config)");
                Some(backend)
            } else {
                debug!("Pi-Mono backend not available");
                None
            }
        };

        // Create OMP backend if manager is available
        let omp_backend = omp_manager.and_then(|manager| {
            if let (Some(tid), Some(ppath)) = (track_id.clone(), project_path.clone()) {
                let backend = Arc::new(OmpBackend::new(Arc::new(manager.clone()), tid, ppath));
                if backend.is_available() {
                    info!("OMP backend available");
                    Some(backend)
                } else {
                    debug!("OMP backend not available");
                    None
                }
            } else {
                None
            }
        });

        Self {
            pi_mono_backend,
            omp_backend,
            active_backend: Arc::new(RwLock::new(None)),
            cancellation_token: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if Pi-Mono is available
    pub fn is_pi_mono_available(&self) -> bool {
        self.pi_mono_backend
            .as_ref()
            .map(|b| b.is_available())
            .unwrap_or(false)
    }

    /// Check if OMP is available
    pub fn is_omp_available(&self) -> bool {
        self.omp_backend
            .as_ref()
            .map(|b| b.is_available())
            .unwrap_or(false)
    }

    /// Get the preferred available backend type
    pub fn get_preferred_backend(&self) -> Option<BackendType> {
        if self.is_pi_mono_available() {
            Some(BackendType::PiMono)
        } else if self.is_omp_available() {
            Some(BackendType::Omp)
        } else {
            None
        }
    }

    /// Get the currently active backend type
    pub async fn get_active_backend(&self) -> Option<BackendType> {
        *self.active_backend.read().await
    }

    /// Create a cancellation token for the current execution
    pub async fn create_cancellation_token(&self) -> Arc<CancellationToken> {
        let token = Arc::new(CancellationToken::new());
        let mut guard = self.cancellation_token.write().await;
        *guard = Some(token.clone());
        token
    }

    /// Cancel the current execution
    pub async fn cancel_execution(&self) -> bool {
        let guard = self.cancellation_token.read().await;
        if let Some(token) = guard.as_ref() {
            token.cancel();
            info!("Execution cancellation requested");
            true
        } else {
            false
        }
    }

    /// Execute a task using the preferred available backend
    pub async fn execute(&self, task: &str, config: &AgentConfig) -> Result<AgentResult> {
        let cancel = {
            let guard = self.cancellation_token.read().await;
            guard.as_ref().map(|t| (**t).clone())
        };

        // Prefer Pi-Mono, fallback to OMP
        if let Some(ref backend) = self.pi_mono_backend {
            if backend.is_available() {
                let mut active = self.active_backend.write().await;
                *active = Some(BackendType::PiMono);
                drop(active);
                return backend.execute(task, config, cancel).await;
            }
        }

        if let Some(ref backend) = self.omp_backend {
            if backend.is_available() {
                let mut active = self.active_backend.write().await;
                *active = Some(BackendType::Omp);
                drop(active);
                return backend.execute(task, config, cancel).await;
            }
        }

        Err(anyhow!("No agent backend available"))
    }

    /// Execute a task with streaming callbacks
    pub async fn execute_with_streaming(
        &self,
        task: &str,
        config: &AgentConfig,
        callback: StreamCallback,
    ) -> Result<AgentResult> {
        let cancel = {
            let guard = self.cancellation_token.read().await;
            guard.as_ref().map(|t| (**t).clone())
        };

        // Prefer Pi-Mono, fallback to OMP
        if let Some(ref backend) = self.pi_mono_backend {
            if backend.is_available() {
                let mut active = self.active_backend.write().await;
                *active = Some(BackendType::PiMono);
                drop(active);
                return backend
                    .execute_with_streaming(task, config, cancel, callback)
                    .await;
            }
        }

        if let Some(ref backend) = self.omp_backend {
            if backend.is_available() {
                let mut active = self.active_backend.write().await;
                *active = Some(BackendType::Omp);
                drop(active);
                return backend
                    .execute_with_streaming(task, config, cancel, callback)
                    .await;
            }
        }

        Err(anyhow!("No agent backend available"))
    }
}

/// Agent role utilities
pub mod role_utils {
    use super::AgentRole;

    /// Get the next role in the cycle: Scout -> Architect -> Critic -> Kraken -> Scout
    pub fn cycle_role(current: &Option<AgentRole>) -> AgentRole {
        match current {
            None => AgentRole::Scout,
            Some(AgentRole::Scout) => AgentRole::Architect,
            Some(AgentRole::Architect) => AgentRole::Critic,
            Some(AgentRole::Critic) => AgentRole::Kraken,
            Some(AgentRole::Kraken) => AgentRole::Scout,
        }
    }

    /// Get a display name for a role
    pub fn role_display_name(role: &AgentRole) -> &'static str {
        match role {
            AgentRole::Scout => "Scout",
            AgentRole::Architect => "Architect",
            AgentRole::Critic => "Critic",
            AgentRole::Kraken => "Kraken",
        }
    }

    /// Get a description for a role
    pub fn role_description(role: &AgentRole) -> &'static str {
        match role {
            AgentRole::Scout => "Fast reconnaissance and info gathering",
            AgentRole::Architect => "Architecture design and planning",
            AgentRole::Critic => "Code review and quality analysis",
            AgentRole::Kraken => "Implementation and execution",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.model, "claude-3-5-sonnet");
        assert_eq!(config.timeout_secs, 300);
        assert!(config.agent_role.is_none());
    }

    #[test]
    fn test_agent_result_success() {
        let result = AgentResult::success("Done".to_string(), BackendType::PiMono);
        assert!(result.success);
        assert_eq!(result.output, "Done");
        assert!(result.error.is_none());
        assert_eq!(result.backend, BackendType::PiMono);
    }

    #[test]
    fn test_agent_result_failure() {
        let result = AgentResult::failure("Error".to_string(), BackendType::Omp);
        assert!(!result.success);
        assert!(result.error.is_some());
        assert_eq!(result.backend, BackendType::Omp);
    }

    #[test]
    fn test_backend_type_display() {
        assert_eq!(format!("{}", BackendType::PiMono), "Pi-Mono");
        assert_eq!(format!("{}", BackendType::Omp), "OMP");
    }

    #[test]
    fn test_role_utils_cycle() {
        assert_eq!(role_utils::cycle_role(&None), AgentRole::Scout);
        assert_eq!(
            role_utils::cycle_role(&Some(AgentRole::Scout)),
            AgentRole::Architect
        );
        assert_eq!(
            role_utils::cycle_role(&Some(AgentRole::Architect)),
            AgentRole::Critic
        );
        assert_eq!(
            role_utils::cycle_role(&Some(AgentRole::Critic)),
            AgentRole::Kraken
        );
        assert_eq!(
            role_utils::cycle_role(&Some(AgentRole::Kraken)),
            AgentRole::Scout
        );
    }

    #[test]
    fn test_role_utils_display_name() {
        assert_eq!(role_utils::role_display_name(&AgentRole::Scout), "Scout");
        assert_eq!(
            role_utils::role_display_name(&AgentRole::Architect),
            "Architect"
        );
        assert_eq!(role_utils::role_display_name(&AgentRole::Critic), "Critic");
        assert_eq!(role_utils::role_display_name(&AgentRole::Kraken), "Kraken");
    }
}
