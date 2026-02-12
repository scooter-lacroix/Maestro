//! OMP Agent Integration for Conductor
//!
//! Provides the ability to spawn OMP workers for track execution
//! within the Conductor pane.

use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::omp::{OmpBridge, OmpWorkerConfig, OmpWorkerStatus};
/// OMP agent configuration for conductor tracks
#[derive(Debug, Clone)]
pub struct OmpAgentConfig {
    /// Model to use for this agent
    pub model: String,
    /// Tools enabled for this agent
    pub tools: Vec<String>,
    /// Timeout for tool execution
    pub timeout_secs: u64,
    /// Whether to use shared LSP/MCP pools from cockpit
    pub use_shared_pools: bool,
}

impl Default for OmpAgentConfig {
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
            use_shared_pools: true,
        }
    }
}

/// OMP agent instance for a track
pub struct OmpAgent {
    /// Track ID this agent is associated with
    track_id: String,
    /// Project path
    project_path: PathBuf,
    /// Agent configuration
    config: OmpAgentConfig,
    /// Bridge to OMP worker
    bridge: Arc<OmpBridge>,
    /// Whether the agent is currently running
    running: Arc<RwLock<bool>>,
}

impl OmpAgent {
    /// Create a new OMP agent for a track
    pub fn new(track_id: String, project_path: PathBuf, config: OmpAgentConfig) -> Self {
        let worker_config = OmpWorkerConfig {
            session_id: track_id.clone(),
            project_path: project_path.clone(),
            model: config.model.clone(),
            tools: config.tools.clone(),
            response_timeout: std::time::Duration::from_secs(config.timeout_secs),
            ..Default::default()
        };

        Self {
            track_id,
            project_path,
            config,
            bridge: Arc::new(OmpBridge::new(worker_config)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Get the track ID
    pub fn track_id(&self) -> &str {
        &self.track_id
    }

    /// Check if the agent is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Start the agent
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(anyhow!("Agent already running"));
        }

        info!("Starting OMP agent for track: {}", self.track_id);

        // The bridge will lazy-initialize the worker on first use
        *running = true;

        Ok(())
    }

    /// Stop the agent
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }

        info!("Stopping OMP agent for track: {}", self.track_id);

        self.bridge.shutdown().await?;
        *running = false;

        Ok(())
    }

    /// Execute a task using the OMP agent
    pub async fn execute_task(&self, task: &str) -> Result<String> {
        if !self.is_running().await {
            self.start().await?;
        }

        debug!("Executing task on OMP agent: {}", task);

        // Use Python tool for code execution, or direct invoke for other tools
        let result = self
            .bridge
            .execute_python(&format!("print('Task: {}')", task), Some(&self.project_path))
            .await
            .context("Failed to execute task via OMP")?;

        Ok(result)
    }

    /// Get agent status
    pub async fn status(&self) -> Result<crate::omp::OmpWorkerStatus> {
        Ok(self.bridge.status().await)
    }
}

/// Manager for OMP agents in the conductor
pub struct OmpAgentManager {
    /// Active agents (track_id -> agent)
    agents: Arc<RwLock<std::collections::HashMap<String, OmpAgent>>>,
    /// Default configuration
    default_config: OmpAgentConfig,
    /// OMP installation path
    omp_path: PathBuf,
}

impl OmpAgentManager {
    /// Create a new agent manager
    pub fn new(omp_path: Option<PathBuf>) -> Self {
        Self {
            agents: Arc::new(RwLock::new(std::collections::HashMap::new())),
            default_config: OmpAgentConfig::default(),
            omp_path: omp_path.unwrap_or_else(|| PathBuf::from("vendor/oh-my-pi")),
        }
    }

    /// Check if OMP is available
    pub fn is_available(&self) -> bool {
        let worker_path = self.omp_path.join("packages/coding-agent/src/worker.ts");
        worker_path.exists()
    }

    /// Get or create an agent for a track
    pub async fn get_or_create_agent(
        &self,
        track_id: String,
        project_path: PathBuf,
        config: Option<OmpAgentConfig>,
    ) -> Result<Arc<OmpAgent>> {
        let mut agents = self.agents.write().await;

        if let Some(agent) = agents.get(&track_id) {
            return Ok(Arc::new(agent.clone()));
        }

        let config = config.unwrap_or_else(|| self.default_config.clone());
        let agent = OmpAgent::new(track_id.clone(), project_path, config);
        
        agents.insert(track_id, agent.clone());

        Ok(Arc::new(agent))
    }

    /// Stop and remove an agent
    pub async fn remove_agent(&self, track_id: &str) -> Result<()> {
        let mut agents = self.agents.write().await;

        if let Some(agent) = agents.remove(track_id) {
            agent.stop().await?;
        }

        Ok(())
    }

    /// Stop all agents
    pub async fn stop_all(&self) -> Result<()> {
        let agents = self.agents.read().await;

        for (_, agent) in agents.iter() {
            if let Err(e) = agent.stop().await {
                warn!("Failed to stop agent {}: {}", agent.track_id(), e);
            }
        }

        Ok(())
    }

    /// Get all active track IDs
    pub async fn active_tracks(&self) -> Vec<String> {
        let agents = self.agents.read().await;
        agents.keys().cloned().collect()
    }
}

impl Clone for OmpAgent {
    fn clone(&self) -> Self {
        Self {
            track_id: self.track_id.clone(),
            project_path: self.project_path.clone(),
            config: self.config.clone(),
            bridge: self.bridge.clone(),
            running: self.running.clone(),
        }
    }
}
