//! Conductor Observer - Event bridge for session monitoring and steering
//!
//! This module provides the observer pattern for the Conductor, allowing
//! the TUI to subscribe to live session events and send steering actions
//! back to the execution engine.
//!
//! ## Architecture
//!
//! ```text
//! ConductorPane
//!   └── Observer
//!         ├── SessionEventBridge (subscribe/publish events)
//!         ├── ObserverAction (steer execution)
//!         └── FileBasedBridge (persisted session observation)
//! ```

use crate::conductor::model::ConductorEvent;
use anyhow::{anyhow, Context, Result as AnyhowResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Home directory for orchestrate sessions
fn orchestrate_base() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".maestro").join("orchestrate")
}

/// Steering command that can be sent to a running session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum SteeringCommand {
    /// Pause the session
    Pause,
    /// Resume the session
    Resume,
    /// Cancel/stop the session
    Cancel,
    /// Retry current task
    Retry,
    /// Skip current task
    Skip,
    /// Custom steering message
    Message { content: String },
    /// Change agent tool
    SwitchAgent { tool: String },
    /// Change max iterations
    SetMaxIterations { count: u64 },
}

impl SteeringCommand {
    /// Serialize to JSON for writing to steering file
    pub fn to_json(&self) -> AnyhowResult<String> {
        serde_json::to_string(self)
            .context("Failed to serialize steering command")
    }

    /// Parse from JSON
    pub fn from_json(json: &str) -> AnyhowResult<Self> {
        serde_json::from_str(json)
            .context("Failed to parse steering command")
    }
}

/// Session observation state from file-based polling
#[derive(Debug, Clone)]
pub struct ObservedSession {
    /// Session ID
    pub session_id: String,
    /// Track ID
    pub track_id: String,
    /// Current session status
    pub status: leindex_core::orchestrate::model::SessionStatus,
    /// Last observed iteration
    pub current_iteration: u64,
    /// Current task ID
    pub current_task: Option<String>,
    /// When we last observed this session
    pub last_observed: DateTime<Utc>,
    /// Tmux session name (if any)
    pub tmux_session: Option<String>,
    /// Session directory
    pub session_dir: PathBuf,
}

/// Observer actions that can be sent to steer execution
#[derive(Debug, Clone, PartialEq)]
pub enum ObserverAction {
    /// Review the current task's output and state
    ReviewCurrentTask {
        iteration: u64,
        task_id: String,
    },
    /// Request retry of the current task
    RequestRetry {
        task_id: String,
        reason: String,
    },
    /// Request skipping the current task
    RequestSkip {
        task_id: String,
        reason: String,
    },
    /// Inject guidance into the current execution
    InjectGuidance {
        task_id: String,
        guidance: String,
    },
}

impl ObserverAction {
    /// Get the task ID this action targets
    pub fn task_id(&self) -> &str {
        match self {
            ObserverAction::ReviewCurrentTask { task_id, .. } => task_id,
            ObserverAction::RequestRetry { task_id, .. } => task_id,
            ObserverAction::RequestSkip { task_id, .. } => task_id,
            ObserverAction::InjectGuidance { task_id, .. } => task_id,
        }
    }

    /// Check if this action requires stopping execution
    pub fn requires_stop(&self) -> bool {
        matches!(self, ObserverAction::RequestRetry { .. } | ObserverAction::RequestSkip { .. })
    }
}

/// Event channel capacity for each session
const EVENT_CHANNEL_CAPACITY: usize = 100;

/// Action channel capacity for sending observer commands
const ACTION_CHANNEL_CAPACITY: usize = 50;

/// Session-specific event channels
type EventChannels = Arc<RwLock<HashMap<String, mpsc::Sender<ConductorEvent>>>>;

/// Trait for bridging session events between the execution engine and observers
#[async_trait::async_trait]
pub trait SessionEventBridge: Send + Sync {
    /// Subscribe to events for a given session
    ///
    /// Returns a receiver that will get events as they are published.
    /// If the session doesn't exist yet, returns an empty channel that
    /// will start receiving events once the session is created.
    fn subscribe(&self, session_id: &str) -> mpsc::Receiver<ConductorEvent>;

    /// Publish an event to all subscribers of a session
    fn publish(&self, session_id: &str, event: ConductorEvent) -> AnyhowResult<()>;

    /// Send an observer action to influence execution
    fn send_action(&self, session_id: &str, action: ObserverAction) -> AnyhowResult<()>;

    /// Check if a session has active subscribers
    fn has_subscribers(&self, session_id: &str) -> bool;

    /// Remove all subscribers for a session (cleanup)
    fn close_session(&self, session_id: &str) -> AnyhowResult<()>;
}

/// In-memory implementation of SessionEventBridge for testing and single-process usage
///
/// This implementation uses tokio channels to deliver events in memory.
/// For multi-process scenarios, this would be replaced with a proper IPC mechanism.
pub struct InMemoryEventBridge {
    /// Event channels per session
    event_channels: EventChannels,
    /// Action channels per session (for sending observer commands)
    action_channels: Arc<RwLock<HashMap<String, mpsc::Sender<ObserverAction>>>>,
}

impl InMemoryEventBridge {
    /// Create a new in-memory event bridge
    pub fn new() -> Self {
        Self {
            event_channels: Arc::new(RwLock::new(HashMap::new())),
            action_channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new event bridge with shared channels
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Ensure an event channel exists for the given session (blocking)
    fn ensure_event_channel_blocking(&self, session_id: &str) -> mpsc::Sender<ConductorEvent> {
        let mut channels = self.event_channels.blocking_write();
        if !channels.contains_key(session_id) {
            let (tx, _rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
            channels.insert(session_id.to_string(), tx);
        }
        channels.get(session_id).unwrap().clone()
    }

    /// Ensure an action channel exists for the given session (blocking)
    fn ensure_action_channel_blocking(&self, session_id: &str) -> mpsc::Sender<ObserverAction> {
        let mut channels = self.action_channels.blocking_write();
        if !channels.contains_key(session_id) {
            let (tx, _rx) = mpsc::channel(ACTION_CHANNEL_CAPACITY);
            channels.insert(session_id.to_string(), tx);
        }
        channels.get(session_id).unwrap().clone()
    }
}

impl Default for InMemoryEventBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SessionEventBridge for InMemoryEventBridge {
    fn subscribe(&self, session_id: &str) -> mpsc::Receiver<ConductorEvent> {
        // Ensure channel exists and get a clone of the sender
        let _tx = self.ensure_event_channel_blocking(session_id);

        // Create a new receiver for this subscription
        let (_, rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
        rx
    }

    fn publish(&self, session_id: &str, event: ConductorEvent) -> AnyhowResult<()> {
        let channels = self.event_channels.try_read()?;
        if let Some(tx) = channels.get(session_id) {
            // Try to send the event, but don't block if channel is full
            if tx.try_send(event).is_err() {
                // Channel is full or closed, log but don't fail
                tracing::debug!(
                    "Failed to send event to session {}: channel full or closed",
                    session_id
                );
            }
        } else {
            // No channel exists yet for this session
            tracing::debug!("No event channel for session {}", session_id);
        }
        Ok(())
    }

    fn send_action(&self, session_id: &str, action: ObserverAction) -> AnyhowResult<()> {
        // Use blocking version to avoid creating a new runtime
        let tx = self.ensure_action_channel_blocking(session_id);
        if let Err(_) = tx.blocking_send(action) {
            tracing::warn!("Failed to send action to session {}: channel closed", session_id);
            Err(anyhow::anyhow!("Action channel closed for session {}", session_id))
        } else {
            Ok(())
        }
    }

    fn has_subscribers(&self, session_id: &str) -> bool {
        let channels = self.event_channels.blocking_read();
        channels.contains_key(session_id)
    }

    fn close_session(&self, session_id: &str) -> AnyhowResult<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let mut event_channels = self.event_channels.write().await;
            event_channels.remove(session_id);

            let mut action_channels = self.action_channels.write().await;
            action_channels.remove(session_id);

            Ok(())
        })
    }
}

/// Observer state attached to a ConductorPane
#[derive(Debug, Clone, Default)]
pub struct ObserverState {
    /// Currently observed session
    pub session_id: Option<String>,
    /// Whether observer mode is active
    pub is_active: bool,
    /// Pending actions to send
    pub pending_actions: Vec<ObserverAction>,
    /// Last observed event (for display)
    pub last_event: Option<ConductorEvent>,
}

impl ObserverState {
    /// Create a new observer state
    pub fn new() -> Self {
        Self::default()
    }

    /// Start observing a session
    pub fn start_observing(&mut self, session_id: String) {
        self.session_id = Some(session_id);
        self.is_active = true;
        self.pending_actions.clear();
    }

    /// Stop observing
    pub fn stop_observing(&mut self) {
        self.session_id = None;
        self.is_active = false;
        self.pending_actions.clear();
    }

    /// Add a pending action to send
    pub fn queue_action(&mut self, action: ObserverAction) {
        self.pending_actions.push(action);
    }

    /// Clear pending actions
    pub fn clear_pending(&mut self) {
        self.pending_actions.clear();
    }
}

/// File-based session observer for orchestrate/implement sessions
///
/// This provides polling-based observation of sessions running in subprocesses,
/// complementing the in-memory bridge for same-process events.
pub struct FileBasedObserver {
    /// Base directory for orchestrate sessions
    base_dir: PathBuf,
    /// Observed sessions
    sessions: Arc<RwLock<HashMap<String, ObservedSession>>>,
    /// Tmux multiplexer for session attachment
    tmux: leindex_core::multiplexer::TmuxMultiplexer,
}

impl FileBasedObserver {
    /// Create a new file-based observer
    pub fn new() -> Self {
        Self {
            base_dir: orchestrate_base(),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            tmux: leindex_core::multiplexer::TmuxMultiplexer::new(),
        }
    }

    /// Create with custom base directory
    pub fn with_base(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            tmux: leindex_core::multiplexer::TmuxMultiplexer::new(),
        }
    }

    /// Discover all active sessions
    pub async fn discover_sessions(&self) -> Vec<String> {
        let mut discovered = Vec::new();

        if !self.base_dir.exists() {
            return discovered;
        }

        let entries = match std::fs::read_dir(&self.base_dir) {
            Ok(entries) => entries,
            Err(_) => return discovered,
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let session_json = path.join("session.json");
            if session_json.exists() {
                if let Some(track_id) = path.file_name().and_then(|n| n.to_str()) {
                    discovered.push(track_id.to_string());
                }
            }
        }

        discovered
    }

    /// Observe a specific track/session
    pub async fn observe_session(&self, track_id: &str) -> AnyhowResult<ObservedSession> {
        let session_dir = self.base_dir.join(track_id);

        if !session_dir.exists() {
            return Err(anyhow!("Session directory does not exist: {:?}", session_dir));
        }

        let session_json = session_dir.join("session.json");
        if !session_json.exists() {
            return Err(anyhow!("session.json does not exist for track: {}", track_id));
        }

        // Read session state
        let content = std::fs::read_to_string(&session_json)
            .context("Failed to read session.json")?;
        let session_state: leindex_core::orchestrate::model::SessionState =
            serde_json::from_str(&content)
                .context("Failed to parse session.json")?;

        // Check for tmux session
        let tmux_session = self.find_tmux_session(track_id).await;

        let observed = ObservedSession {
            session_id: session_state.session_id.clone(),
            track_id: track_id.to_string(),
            status: session_state.status,
            current_iteration: session_state.current_iteration,
            current_task: session_state.current_task_id.clone(),
            last_observed: Utc::now(),
            tmux_session,
            session_dir,
        };

        // Store in sessions map
        let mut sessions = self.sessions.write().await;
        sessions.insert(observed.session_id.clone(), observed.clone());

        Ok(observed)
    }

    /// Stop observing a session
    pub async fn unobserve_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
    }

    /// Send a steering command to a session
    pub async fn send_steering(&self, session_id: &str, command: SteeringCommand) -> AnyhowResult<()> {
        let session_dir = {
            let sessions = self.sessions.read().await;
            let session = sessions.get(session_id)
                .ok_or_else(|| anyhow!("Session not found: {}", session_id))?;
            session.session_dir.clone()
        };

        // Append to steering.jsonl
        let steering_path = session_dir.join("steering.jsonl");
        let command_json = command.to_json()?;

        // Create parent directories if needed
        if let Some(parent) = steering_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Append to file
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&steering_path)
            .context("Failed to open steering file")?;

        use std::io::Write;
        writeln!(file, "{}", command_json)
            .context("Failed to write steering command")?;

        tracing::info!("Sent steering command to session {}: {:?}", session_id, command);
        Ok(())
    }

    /// Attach to a session's tmux session (returns session name)
    pub async fn attach_tmux(&self, session_id: &str) -> AnyhowResult<String> {
        let tmux_session = {
            let sessions = self.sessions.read().await;
            let session = sessions.get(session_id)
                .ok_or_else(|| anyhow!("Session not found: {}", session_id))?;
            session.tmux_session.clone()
                .ok_or_else(|| anyhow!("No tmux session for: {}", session_id))?
        };

        // Check if session still exists
        if !self.tmux.session_exists(&tmux_session) {
            return Err(anyhow!("Tmux session no longer exists: {}", tmux_session));
        }

        Ok(tmux_session)
    }

    /// Get pane content from a tmux session
    pub async fn get_tmux_content(&self, session_id: &str, lines: usize) -> AnyhowResult<String> {
        let tmux_session = self.attach_tmux(session_id).await?;
        leindex_core::multiplexer::TmuxMultiplexer::get_pane_content(&tmux_session, lines)
    }

    /// Find the tmux session for a track
    async fn find_tmux_session(&self, track_id: &str) -> Option<String> {
        // Refresh tmux session cache
        let _ = self.tmux.refresh_session_cache();

        // List all maestro sessions
        let maestro_sessions = self.tmux.list_maestro_sessions();

        // Look for a session that contains our track_id
        for session_name in maestro_sessions {
            // Session names are like "maestro_track-id_12345678"
            if session_name.contains(track_id) || session_name.replace('_', "-").contains(track_id) {
                return Some(session_name);
            }
        }

        None
    }

    /// Get all observed sessions
    pub async fn get_sessions(&self) -> Vec<ObservedSession> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// Get a specific observed session
    pub async fn get_session(&self, session_id: &str) -> Option<ObservedSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Poll all observed sessions for state changes
    pub async fn poll_sessions(&mut self) -> Vec<ObservedSession> {
        let mut updated = Vec::new();

        let track_ids: Vec<String> = {
            let sessions = self.sessions.read().await;
            sessions.values().map(|s| s.track_id.clone()).collect()
        };

        for track_id in track_ids {
            if let Ok(observed) = self.observe_session(&track_id).await {
                updated.push(observed);
            }
        }

        updated
    }
}

impl Default for FileBasedObserver {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait to convert ObserverAction to SteeringCommand
pub trait ToSteeringCommand {
    fn to_steering_command(&self) -> Option<SteeringCommand>;
}

impl ToSteeringCommand for ObserverAction {
    fn to_steering_command(&self) -> Option<SteeringCommand> {
        match self {
            ObserverAction::RequestRetry { .. } => Some(SteeringCommand::Retry),
            ObserverAction::RequestSkip { .. } => Some(SteeringCommand::Skip),
            ObserverAction::InjectGuidance { guidance, .. } => {
                Some(SteeringCommand::Message {
                    content: guidance.clone(),
                })
            }
            ObserverAction::ReviewCurrentTask { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conductor::model::ConductorEvent;

    #[test]
    fn test_observer_action_task_id() {
        let action = ObserverAction::RequestRetry {
            task_id: "task-123".to_string(),
            reason: "Network error".to_string(),
        };
        assert_eq!(action.task_id(), "task-123");
    }

    #[test]
    fn test_observer_action_requires_stop() {
        let retry = ObserverAction::RequestRetry {
            task_id: "task-1".to_string(),
            reason: "Error".to_string(),
        };
        assert!(retry.requires_stop());

        let skip = ObserverAction::RequestSkip {
            task_id: "task-1".to_string(),
            reason: "Blocked".to_string(),
        };
        assert!(skip.requires_stop());

        let review = ObserverAction::ReviewCurrentTask {
            iteration: 1,
            task_id: "task-1".to_string(),
        };
        assert!(!review.requires_stop());

        let guidance = ObserverAction::InjectGuidance {
            task_id: "task-1".to_string(),
            guidance: "Use async".to_string(),
        };
        assert!(!guidance.requires_stop());
    }

    #[test]
    fn test_observer_state_lifecycle() {
        let mut state = ObserverState::new();
        assert!(!state.is_active);
        assert!(state.session_id.is_none());

        state.start_observing("session-1".to_string());
        assert!(state.is_active);
        assert_eq!(state.session_id.as_ref().unwrap(), "session-1");

        state.queue_action(ObserverAction::ReviewCurrentTask {
            iteration: 1,
            task_id: "task-1".to_string(),
        });
        assert_eq!(state.pending_actions.len(), 1);

        state.stop_observing();
        assert!(!state.is_active);
        assert!(state.session_id.is_none());
        assert!(state.pending_actions.is_empty());
    }

    #[tokio::test]
    async fn test_event_bridge_has_subscribers() {
        let bridge = InMemoryEventBridge::new();
        assert!(!bridge.has_subscribers("nonexistent"));

        // Subscribe creates a channel
        let _rx = bridge.subscribe("test-session");
        assert!(bridge.has_subscribers("test-session"));

        // Close session removes channel
        bridge.close_session("test-session").unwrap();
        assert!(!bridge.has_subscribers("test-session"));
    }

    #[tokio::test]
    async fn test_event_bridge_publish_to_nonexistent_session() {
        let bridge = InMemoryEventBridge::new();
        let event = ConductorEvent::IterationStarted {
            iteration: 1,
            task_id: "task-1".to_string(),
        };

        // Should not error even if session doesn't exist
        assert!(bridge.publish("nonexistent", event).is_ok());
    }

    #[test]
    fn test_steering_command_serialization() {
        let cmd = SteeringCommand::Pause;
        let json = cmd.to_json().unwrap();
        assert!(json.contains("\"action\""));
        assert!(json.contains("pause"));

        let parsed = SteeringCommand::from_json(&json).unwrap();
        match parsed {
            SteeringCommand::Pause => {}
            _ => panic!("Expected Pause, got {:?}", parsed),
        }
    }

    #[test]
    fn test_steering_command_message() {
        let cmd = SteeringCommand::Message {
            content: "Test message".to_string(),
        };
        let json = cmd.to_json().unwrap();
        let parsed = SteeringCommand::from_json(&json).unwrap();

        match parsed {
            SteeringCommand::Message { content } => {
                assert_eq!(content, "Test message");
            }
            _ => panic!("Expected Message, got {:?}", parsed),
        }
    }

    #[test]
    fn test_steering_command_switch_agent() {
        let cmd = SteeringCommand::SwitchAgent {
            tool: "claude".to_string(),
        };
        let json = cmd.to_json().unwrap();
        let parsed = SteeringCommand::from_json(&json).unwrap();

        match parsed {
            SteeringCommand::SwitchAgent { tool } => {
                assert_eq!(tool, "claude");
            }
            _ => panic!("Expected SwitchAgent, got {:?}", parsed),
        }
    }

    #[test]
    fn test_observer_action_to_steering_command() {
        let retry = ObserverAction::RequestRetry {
            task_id: "task-1".to_string(),
            reason: "Error".to_string(),
        };
        assert!(matches!(retry.to_steering_command(), Some(SteeringCommand::Retry)));

        let skip = ObserverAction::RequestSkip {
            task_id: "task-1".to_string(),
            reason: "Blocked".to_string(),
        };
        assert!(matches!(skip.to_steering_command(), Some(SteeringCommand::Skip)));

        let guidance = ObserverAction::InjectGuidance {
            task_id: "task-1".to_string(),
            guidance: "Use async".to_string(),
        };
        match guidance.to_steering_command() {
            Some(SteeringCommand::Message { content }) => {
                assert_eq!(content, "Use async");
            }
            _ => panic!("Expected Message command"),
        }

        let review = ObserverAction::ReviewCurrentTask {
            iteration: 1,
            task_id: "task-1".to_string(),
        };
        assert!(review.to_steering_command().is_none());
    }

    #[tokio::test]
    async fn test_file_based_observer_discover_empty() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let observer = FileBasedObserver::with_base(temp_dir.path().to_path_buf());

        let sessions = observer.discover_sessions().await;
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_file_based_observer_discover_with_session() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let track_dir = temp_dir.path().join("test-track");
        std::fs::create_dir_all(&track_dir).unwrap();

        // Create a minimal session.json
        let session_json = r#"{
            "session_id": "test-session",
            "track_id": "test-track",
            "status": "running",
            "mode": "building",
            "current_iteration": 1,
            "current_task_id": null,
            "started_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
            "agent_config": {
                "tool": "claude",
                "model": null,
                "sandbox": false,
                "dangerous_mode": false
            }
        }"#;

        std::fs::write(track_dir.join("session.json"), session_json).unwrap();

        let observer = FileBasedObserver::with_base(temp_dir.path().to_path_buf());
        let sessions = observer.discover_sessions().await;

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0], "test-track");
    }

    #[tokio::test]
    async fn test_file_based_observer_observe_session() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let track_dir = temp_dir.path().join("test-track");
        std::fs::create_dir_all(&track_dir).unwrap();

        // Create a minimal session.json
        let session_json = r#"{
            "session_id": "test-session",
            "track_id": "test-track",
            "status": "running",
            "mode": "building",
            "current_iteration": 5,
            "current_task_id": "task-1",
            "started_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
            "agent_config": {
                "tool": "claude",
                "model": null,
                "sandbox": false,
                "dangerous_mode": false
            }
        }"#;

        std::fs::write(track_dir.join("session.json"), session_json).unwrap();

        let observer = FileBasedObserver::with_base(temp_dir.path().to_path_buf());
        let observed = observer.observe_session("test-track").await.unwrap();

        assert_eq!(observed.session_id, "test-session");
        assert_eq!(observed.track_id, "test-track");
        assert_eq!(observed.current_iteration, 5);
        assert_eq!(observed.current_task.as_ref().unwrap(), "task-1");
    }

    #[tokio::test]
    async fn test_file_based_observer_send_steering() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let track_dir = temp_dir.path().join("test-track");
        std::fs::create_dir_all(&track_dir).unwrap();

        // Create a minimal session.json
        let session_json = r#"{
            "session_id": "test-session",
            "track_id": "test-track",
            "status": "running",
            "mode": "building",
            "current_iteration": 1,
            "current_task_id": null,
            "started_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
            "agent_config": {
                "tool": "claude",
                "model": null,
                "sandbox": false,
                "dangerous_mode": false
            }
        }"#;

        std::fs::write(track_dir.join("session.json"), session_json).unwrap();

        let observer = FileBasedObserver::with_base(temp_dir.path().to_path_buf());
        observer.observe_session("test-track").await.unwrap();

        // Send a steering command
        observer.send_steering("test-session", SteeringCommand::Retry).await.unwrap();

        // Check that steering.jsonl was created with the command
        let steering_path = track_dir.join("steering.jsonl");
        assert!(steering_path.exists());

        let content = std::fs::read_to_string(&steering_path).unwrap();
        assert!(content.contains("\"action\""));
        assert!(content.contains("retry"));
    }

    #[tokio::test]
    async fn test_file_based_observer_unobserve_session() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let track_dir = temp_dir.path().join("test-track");
        std::fs::create_dir_all(&track_dir).unwrap();

        // Create a minimal session.json
        let session_json = r#"{
            "session_id": "test-session",
            "track_id": "test-track",
            "status": "running",
            "mode": "building",
            "current_iteration": 1,
            "current_task_id": null,
            "started_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
            "agent_config": {
                "tool": "claude",
                "model": null,
                "sandbox": false,
                "dangerous_mode": false
            }
        }"#;

        std::fs::write(track_dir.join("session.json"), session_json).unwrap();

        let observer = FileBasedObserver::with_base(temp_dir.path().to_path_buf());
        observer.observe_session("test-track").await.unwrap();

        // Verify session is tracked
        let sessions = observer.get_sessions().await;
        assert_eq!(sessions.len(), 1);

        // Unobserve
        observer.unobserve_session("test-session").await;

        // Verify session is removed
        let sessions = observer.get_sessions().await;
        assert_eq!(sessions.len(), 0);
    }
}
