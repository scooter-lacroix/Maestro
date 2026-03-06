//! Lightweight observability backends for MaestroClaw.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TelemetryCorrelation {
    pub session_id: Option<String>,
    pub thread_id: Option<String>,
    pub turn_index: Option<usize>,
    pub tool_call_id: Option<String>,
    pub principal: Option<String>,
    pub sender: Option<String>,
    pub surface: Option<String>,
    pub component: Option<String>,
}

impl TelemetryCorrelation {
    pub fn with_surface(mut self, surface: impl Into<String>) -> Self {
        self.surface = Some(surface.into());
        self
    }

    pub fn with_component(mut self, component: impl Into<String>) -> Self {
        self.component = Some(component.into());
        self
    }

    pub fn with_principal(mut self, principal: impl Into<String>) -> Self {
        self.principal = Some(principal.into());
        self
    }

    pub fn with_sender(mut self, sender: impl Into<String>) -> Self {
        self.sender = Some(sender.into());
        self
    }

    pub fn inferred_surface(&self) -> Option<String> {
        self.surface.clone().or_else(|| {
            self.component
                .as_deref()
                .map(|component| component.split(':').next().unwrap_or(component).to_string())
        })
    }

    pub fn actor(&self) -> Option<String> {
        self.principal.clone().or_else(|| self.sender.clone())
    }

    pub fn normalized_for_component(mut self, component: &str) -> Self {
        if self.component.is_none() {
            self.component = Some(component.to_string());
        }
        if self.surface.is_none() {
            self.surface = Some(component.split(':').next().unwrap_or(component).to_string());
        }
        self
    }

    pub fn normalized_with(
        mut self,
        session_id: Option<String>,
        component: Option<String>,
        surface_hint: Option<&str>,
    ) -> Self {
        if self.session_id.is_none() {
            self.session_id = session_id;
        }
        if self.component.is_none() {
            self.component = component;
        }
        if self.surface.is_none() {
            self.surface = surface_hint
                .map(str::to_string)
                .or_else(|| self.inferred_surface());
        }
        self
    }

    pub fn matches(&self, filter: &TelemetryCorrelation) -> bool {
        field_matches(&self.session_id, &filter.session_id)
            && field_matches(&self.thread_id, &filter.thread_id)
            && value_matches(self.turn_index, filter.turn_index)
            && field_matches(&self.tool_call_id, &filter.tool_call_id)
            && field_matches(&self.principal, &filter.principal)
            && field_matches(&self.sender, &filter.sender)
            && field_matches(&self.inferred_surface(), &filter.inferred_surface())
            && field_matches(&self.component, &filter.component)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichObserverEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: ObserverEventType,
    pub component: String,
    #[serde(default)]
    pub correlation: TelemetryCorrelation,
    pub metadata: EventMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ObserverEventType {
    AgentStart,
    AgentComplete,
    AgentError,
    SchedulerTick,
    SchedulerJobStart,
    SchedulerJobComplete,
    SchedulerJobError,
    HeartbeatTick,
    HeartbeatTaskStart,
    HeartbeatTaskComplete,
    HeartbeatTaskError,
    ChannelMessage,
    ChannelResponse,
    ChannelError,
    RuntimeStart,
    RuntimeStop,
    RuntimeError,
    DaemonStart,
    DaemonStop,
    ComponentHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventMetadata {
    pub channel: Option<String>,
    pub tool: Option<String>,
    pub duration_ms: Option<i64>,
    pub prompt_chars: Option<usize>,
    pub response_chars: Option<usize>,
    pub error: Option<String>,
    pub job_id: Option<String>,
    pub job_type: Option<String>,
    pub task_name: Option<String>,
    pub message_id: Option<String>,
    pub sender: Option<String>,
    pub principal: Option<String>,
    pub success: Option<bool>,
    pub restart_count: Option<u32>,
    pub health_status: Option<String>,
    pub workspace_dir: Option<String>,
    pub surface: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ObserverEvent {
    AgentStart {
        tool: String,
    },
    AgentComplete {
        tool: String,
        duration_ms: i64,
    },
    AgentError {
        tool: String,
        error: String,
    },
    SchedulerTick,
    SchedulerJobStart {
        job_id: String,
        job_type: String,
    },
    SchedulerJobComplete {
        job_id: String,
        job_type: String,
        duration_ms: i64,
        success: bool,
    },
    SchedulerJobError {
        job_id: String,
        job_type: String,
        error: String,
    },
    HeartbeatTick,
    HeartbeatTaskStart {
        task_name: String,
    },
    HeartbeatTaskComplete {
        task_name: String,
        duration_ms: i64,
        success: bool,
    },
    HeartbeatTaskError {
        task_name: String,
        error: String,
    },
    ChannelMessage {
        channel: String,
        sender: String,
        message_id: String,
    },
    ChannelResponse {
        channel: String,
        message_id: String,
        success: bool,
    },
    ChannelError {
        channel: String,
        error: String,
    },
    RuntimeStart {
        workspace_dir: String,
    },
    RuntimeStop,
    RuntimeError {
        error: String,
    },
    DaemonStart,
    DaemonStop,
    ComponentHealth {
        component: String,
        healthy: bool,
        restart_count: u32,
    },
}

impl ObserverEvent {
    pub fn component_name(&self) -> String {
        match self {
            ObserverEvent::AgentStart { tool }
            | ObserverEvent::AgentComplete { tool, .. }
            | ObserverEvent::AgentError { tool, .. } => {
                if tool.is_empty() {
                    "agent".to_string()
                } else {
                    format!("agent:{tool}")
                }
            }
            ObserverEvent::SchedulerTick
            | ObserverEvent::SchedulerJobStart { .. }
            | ObserverEvent::SchedulerJobComplete { .. }
            | ObserverEvent::SchedulerJobError { .. } => "scheduler".to_string(),
            ObserverEvent::HeartbeatTick
            | ObserverEvent::HeartbeatTaskStart { .. }
            | ObserverEvent::HeartbeatTaskComplete { .. }
            | ObserverEvent::HeartbeatTaskError { .. } => "heartbeat".to_string(),
            ObserverEvent::ChannelMessage { channel, .. }
            | ObserverEvent::ChannelResponse { channel, .. }
            | ObserverEvent::ChannelError { channel, .. } => {
                if channel.is_empty() {
                    "channel".to_string()
                } else {
                    format!("channel:{channel}")
                }
            }
            ObserverEvent::RuntimeStart { .. }
            | ObserverEvent::RuntimeStop
            | ObserverEvent::RuntimeError { .. } => "runtime".to_string(),
            ObserverEvent::DaemonStart | ObserverEvent::DaemonStop => "daemon".to_string(),
            ObserverEvent::ComponentHealth { component, .. } => component.clone(),
        }
    }

    pub fn to_rich_event(&self, component: &str) -> RichObserverEvent {
        self.to_rich_event_with_correlation(component, TelemetryCorrelation::default())
    }

    pub fn to_rich_event_with_correlation(
        &self,
        component: &str,
        correlation: TelemetryCorrelation,
    ) -> RichObserverEvent {
        let (event_type, metadata) = match self {
            ObserverEvent::AgentStart { tool } => (
                ObserverEventType::AgentStart,
                EventMetadata {
                    tool: Some(tool.clone()),
                    ..Default::default()
                },
            ),
            ObserverEvent::AgentComplete { tool, duration_ms } => (
                ObserverEventType::AgentComplete,
                EventMetadata {
                    tool: Some(tool.clone()),
                    duration_ms: Some(*duration_ms),
                    success: Some(true),
                    ..Default::default()
                },
            ),
            ObserverEvent::AgentError { tool, error } => (
                ObserverEventType::AgentError,
                EventMetadata {
                    tool: Some(tool.clone()),
                    error: Some(error.clone()),
                    success: Some(false),
                    ..Default::default()
                },
            ),
            ObserverEvent::SchedulerTick => {
                (ObserverEventType::SchedulerTick, EventMetadata::default())
            }
            ObserverEvent::SchedulerJobStart { job_id, job_type } => (
                ObserverEventType::SchedulerJobStart,
                EventMetadata {
                    job_id: Some(job_id.clone()),
                    job_type: Some(job_type.clone()),
                    ..Default::default()
                },
            ),
            ObserverEvent::SchedulerJobComplete {
                job_id,
                job_type,
                duration_ms,
                success,
            } => (
                ObserverEventType::SchedulerJobComplete,
                EventMetadata {
                    job_id: Some(job_id.clone()),
                    job_type: Some(job_type.clone()),
                    duration_ms: Some(*duration_ms),
                    success: Some(*success),
                    ..Default::default()
                },
            ),
            ObserverEvent::SchedulerJobError {
                job_id,
                job_type,
                error,
            } => (
                ObserverEventType::SchedulerJobError,
                EventMetadata {
                    job_id: Some(job_id.clone()),
                    job_type: Some(job_type.clone()),
                    error: Some(error.clone()),
                    success: Some(false),
                    ..Default::default()
                },
            ),
            ObserverEvent::HeartbeatTick => {
                (ObserverEventType::HeartbeatTick, EventMetadata::default())
            }
            ObserverEvent::HeartbeatTaskStart { task_name } => (
                ObserverEventType::HeartbeatTaskStart,
                EventMetadata {
                    task_name: Some(task_name.clone()),
                    ..Default::default()
                },
            ),
            ObserverEvent::HeartbeatTaskComplete {
                task_name,
                duration_ms,
                success,
            } => (
                ObserverEventType::HeartbeatTaskComplete,
                EventMetadata {
                    task_name: Some(task_name.clone()),
                    duration_ms: Some(*duration_ms),
                    success: Some(*success),
                    ..Default::default()
                },
            ),
            ObserverEvent::HeartbeatTaskError { task_name, error } => (
                ObserverEventType::HeartbeatTaskError,
                EventMetadata {
                    task_name: Some(task_name.clone()),
                    error: Some(error.clone()),
                    success: Some(false),
                    ..Default::default()
                },
            ),
            ObserverEvent::ChannelMessage {
                channel,
                sender,
                message_id,
            } => (
                ObserverEventType::ChannelMessage,
                EventMetadata {
                    channel: Some(channel.clone()),
                    sender: Some(sender.clone()),
                    principal: Some(sender.clone()),
                    message_id: Some(message_id.clone()),
                    surface: Some("channel".to_string()),
                    ..Default::default()
                },
            ),
            ObserverEvent::ChannelResponse {
                channel,
                message_id,
                success,
            } => (
                ObserverEventType::ChannelResponse,
                EventMetadata {
                    channel: Some(channel.clone()),
                    message_id: Some(message_id.clone()),
                    success: Some(*success),
                    surface: Some("channel".to_string()),
                    ..Default::default()
                },
            ),
            ObserverEvent::ChannelError { channel, error } => (
                ObserverEventType::ChannelError,
                EventMetadata {
                    channel: Some(channel.clone()),
                    error: Some(error.clone()),
                    success: Some(false),
                    surface: Some("channel".to_string()),
                    ..Default::default()
                },
            ),
            ObserverEvent::RuntimeStart { workspace_dir } => (
                ObserverEventType::RuntimeStart,
                EventMetadata {
                    workspace_dir: Some(workspace_dir.clone()),
                    surface: Some("runtime".to_string()),
                    ..Default::default()
                },
            ),
            ObserverEvent::RuntimeStop => {
                (ObserverEventType::RuntimeStop, EventMetadata::default())
            }
            ObserverEvent::RuntimeError { error } => (
                ObserverEventType::RuntimeError,
                EventMetadata {
                    error: Some(error.clone()),
                    success: Some(false),
                    ..Default::default()
                },
            ),
            ObserverEvent::DaemonStart => {
                (ObserverEventType::DaemonStart, EventMetadata::default())
            }
            ObserverEvent::DaemonStop => (ObserverEventType::DaemonStop, EventMetadata::default()),
            ObserverEvent::ComponentHealth {
                component: _,
                healthy,
                restart_count,
            } => (
                ObserverEventType::ComponentHealth,
                EventMetadata {
                    health_status: Some(if *healthy {
                        "healthy".to_string()
                    } else {
                        "unhealthy".to_string()
                    }),
                    restart_count: Some(*restart_count),
                    ..Default::default()
                },
            ),
        };

        RichObserverEvent {
            timestamp: Utc::now(),
            event_type,
            component: component.to_string(),
            correlation: correlation.normalized_for_component(component),
            metadata,
        }
    }

    pub fn to_rich_event_auto(&self) -> RichObserverEvent {
        let component = self.component_name();
        self.to_rich_event(&component)
    }

    pub fn to_rich_event_auto_with_correlation(
        &self,
        correlation: TelemetryCorrelation,
    ) -> RichObserverEvent {
        let component = self.component_name();
        self.to_rich_event_with_correlation(&component, correlation)
    }
}

pub trait Observer: Send + Sync {
    fn name(&self) -> &str;
    fn record_event(&self, event: &ObserverEvent) {
        self.record_correlated_event(event, TelemetryCorrelation::default());
    }
    fn record_correlated_event(&self, event: &ObserverEvent, correlation: TelemetryCorrelation);
    fn clone_box(&self) -> Box<dyn Observer>;
}

impl Clone for Box<dyn Observer> {
    fn clone(&self) -> Box<dyn Observer> {
        self.clone_box()
    }
}

pub struct NoopObserver;

impl Observer for NoopObserver {
    fn name(&self) -> &str {
        "noop"
    }

    fn record_correlated_event(&self, _event: &ObserverEvent, _correlation: TelemetryCorrelation) {}

    fn clone_box(&self) -> Box<dyn Observer> {
        Box::new(NoopObserver)
    }
}

pub struct LogObserver;

impl Observer for LogObserver {
    fn name(&self) -> &str {
        "log"
    }

    fn record_correlated_event(&self, event: &ObserverEvent, correlation: TelemetryCorrelation) {
        let session_id = correlation.session_id.as_deref().unwrap_or("");
        let thread_id = correlation.thread_id.as_deref().unwrap_or("");
        let tool_call_id = correlation.tool_call_id.as_deref().unwrap_or("");
        let principal = correlation.principal.as_deref().unwrap_or("");
        let sender = correlation.sender.as_deref().unwrap_or("");
        let surface = correlation.inferred_surface().unwrap_or_default();
        match event {
            ObserverEvent::AgentStart { tool } => {
                tracing::info!(tool, session_id, thread_id, surface, "agent started")
            }
            ObserverEvent::AgentComplete { tool, duration_ms } => {
                tracing::info!(
                    tool,
                    duration_ms,
                    session_id,
                    thread_id,
                    surface,
                    "agent completed"
                )
            }
            ObserverEvent::AgentError { tool, error } => {
                tracing::error!(tool, error, session_id, thread_id, surface, "agent error")
            }
            ObserverEvent::SchedulerTick => tracing::debug!("scheduler tick"),
            ObserverEvent::SchedulerJobStart { job_id, job_type } => {
                tracing::debug!(job_id, job_type, surface, "scheduler job started")
            }
            ObserverEvent::SchedulerJobComplete {
                job_id,
                duration_ms,
                success,
                ..
            } => {
                tracing::debug!(
                    job_id,
                    duration_ms,
                    success,
                    surface,
                    "scheduler job completed"
                )
            }
            ObserverEvent::SchedulerJobError { job_id, error, .. } => {
                tracing::error!(job_id, error, surface, "scheduler job error")
            }
            ObserverEvent::HeartbeatTick => tracing::debug!("heartbeat tick"),
            ObserverEvent::HeartbeatTaskStart { task_name } => {
                tracing::debug!(task_name, surface, "heartbeat task started")
            }
            ObserverEvent::HeartbeatTaskComplete {
                task_name,
                duration_ms,
                success,
            } => {
                tracing::debug!(
                    task_name,
                    duration_ms,
                    success,
                    surface,
                    "heartbeat task completed"
                )
            }
            ObserverEvent::HeartbeatTaskError { task_name, error } => {
                tracing::error!(task_name, error, surface, "heartbeat task error")
            }
            ObserverEvent::ChannelMessage { channel, .. } => {
                tracing::debug!(
                    channel,
                    sender,
                    principal,
                    tool_call_id,
                    surface,
                    "channel message received"
                )
            }
            ObserverEvent::ChannelResponse {
                channel, success, ..
            } => {
                tracing::debug!(
                    channel,
                    success,
                    principal,
                    surface,
                    "channel response sent"
                )
            }
            ObserverEvent::ChannelError { channel, error } => {
                tracing::error!(channel, error, principal, surface, "channel error")
            }
            ObserverEvent::RuntimeStart { workspace_dir } => {
                tracing::info!(workspace_dir, surface, "runtime started")
            }
            ObserverEvent::RuntimeStop => tracing::info!("runtime stopped"),
            ObserverEvent::RuntimeError { error } => {
                tracing::error!(error, surface, "runtime error")
            }
            ObserverEvent::DaemonStart => tracing::info!("daemon started"),
            ObserverEvent::DaemonStop => tracing::info!("daemon stopped"),
            ObserverEvent::ComponentHealth {
                component,
                healthy,
                restart_count,
            } => {
                tracing::debug!(component, healthy, restart_count, "component health")
            }
        }
    }

    fn clone_box(&self) -> Box<dyn Observer> {
        Box::new(LogObserver)
    }
}

pub struct PersistentObserver {
    storage_path: PathBuf,
    max_events: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EventCountBreakdown {
    pub key: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObservabilityReport {
    pub total_events: usize,
    pub recent_events: Vec<RichObserverEvent>,
    pub by_component: Vec<EventCountBreakdown>,
    pub by_surface: Vec<EventCountBreakdown>,
    pub by_principal: Vec<EventCountBreakdown>,
    pub by_session: Vec<EventCountBreakdown>,
    pub by_thread: Vec<EventCountBreakdown>,
    pub by_tool_call: Vec<EventCountBreakdown>,
    pub by_event_type: Vec<EventCountBreakdown>,
}

impl PersistentObserver {
    fn from_storage_path(storage_path: PathBuf, max_events: usize) -> Self {
        Self {
            storage_path,
            max_events,
        }
    }

    pub fn new(workspace_dir: &Path) -> Self {
        Self::from_storage_path(
            workspace_dir.join("observability").join("events.jsonl"),
            10000,
        )
    }

    pub fn with_max_events(workspace_dir: &Path, max_events: usize) -> Self {
        Self::from_storage_path(
            workspace_dir.join("observability").join("events.jsonl"),
            max_events,
        )
    }

    fn ensure_dir(&self) -> std::io::Result<()> {
        if let Some(parent) = self.storage_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    pub fn query_events(&self, filter: &EventFilter) -> Vec<RichObserverEvent> {
        if !self.storage_path.exists() {
            return Vec::new();
        }

        let content = match std::fs::read_to_string(&self.storage_path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let mut events = Vec::new();
        for line in content.lines() {
            if let Ok(event) = serde_json::from_str::<RichObserverEvent>(line) {
                if filter.matches(&event) {
                    events.push(event);
                }
            }
        }

        events
    }

    pub fn query_events_by_type(&self, event_type: ObserverEventType) -> Vec<RichObserverEvent> {
        let filter = EventFilter {
            event_types: Some(vec![event_type]),
            components: None,
            from_timestamp: None,
            to_timestamp: None,
        };
        self.query_events(&filter)
    }

    pub fn query_events_by_component(&self, component: &str) -> Vec<RichObserverEvent> {
        let filter = EventFilter {
            event_types: None,
            components: Some(vec![component.to_string()]),
            from_timestamp: None,
            to_timestamp: None,
        };
        self.query_events(&filter)
    }

    pub fn query_recent_events(&self, count: usize) -> Vec<RichObserverEvent> {
        if !self.storage_path.exists() {
            return Vec::new();
        }

        let content = match std::fs::read_to_string(&self.storage_path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let mut events: Vec<RichObserverEvent> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        events.truncate(count);
        events
    }

    pub fn query_events_by_correlation(
        &self,
        correlation: &TelemetryCorrelation,
    ) -> Vec<RichObserverEvent> {
        self.query_events(&EventFilter::default())
            .into_iter()
            .filter(|event| event.correlation.matches(correlation))
            .collect()
    }

    pub fn recent_report(&self, count: usize) -> ObservabilityReport {
        let recent_events = self.query_recent_events(count);
        let mut by_component = std::collections::HashMap::<String, usize>::new();
        let mut by_surface = std::collections::HashMap::<String, usize>::new();
        let mut by_principal = std::collections::HashMap::<String, usize>::new();
        let mut by_session = std::collections::HashMap::<String, usize>::new();
        let mut by_thread = std::collections::HashMap::<String, usize>::new();
        let mut by_tool_call = std::collections::HashMap::<String, usize>::new();
        let mut by_event_type = std::collections::HashMap::<String, usize>::new();

        for event in &recent_events {
            *by_component.entry(event.component.clone()).or_insert(0) += 1;
            if let Some(surface) = event.correlation.inferred_surface() {
                *by_surface.entry(surface).or_insert(0) += 1;
            }
            if let Some(principal) = event.correlation.actor() {
                *by_principal.entry(principal).or_insert(0) += 1;
            }
            if let Some(session_id) = event.correlation.session_id.clone() {
                *by_session.entry(session_id).or_insert(0) += 1;
            }
            if let Some(thread_id) = event.correlation.thread_id.clone() {
                *by_thread.entry(thread_id).or_insert(0) += 1;
            }
            if let Some(tool_call_id) = event.correlation.tool_call_id.clone() {
                *by_tool_call.entry(tool_call_id).or_insert(0) += 1;
            }
            *by_event_type
                .entry(format!("{:?}", event.event_type))
                .or_insert(0) += 1;
        }

        ObservabilityReport {
            total_events: self.get_event_count(),
            recent_events,
            by_component: sort_breakdown(by_component),
            by_surface: sort_breakdown(by_surface),
            by_principal: sort_breakdown(by_principal),
            by_session: sort_breakdown(by_session),
            by_thread: sort_breakdown(by_thread),
            by_tool_call: sort_breakdown(by_tool_call),
            by_event_type: sort_breakdown(by_event_type),
        }
    }

    pub fn get_event_count(&self) -> usize {
        if !self.storage_path.exists() {
            return 0;
        }

        std::fs::read_to_string(&self.storage_path)
            .map(|c| c.lines().count())
            .unwrap_or(0)
    }

    pub fn clear_events(&self) -> std::io::Result<()> {
        if self.storage_path.exists() {
            std::fs::remove_file(&self.storage_path)?;
        }
        Ok(())
    }

    fn rotate_if_needed(&self) -> std::io::Result<()> {
        let count = self.get_event_count();
        if count >= self.max_events {
            let archive_path = self.storage_path.with_extension("jsonl.old");
            std::fs::rename(&self.storage_path, archive_path)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    pub event_types: Option<Vec<ObserverEventType>>,
    pub components: Option<Vec<String>>,
    pub from_timestamp: Option<DateTime<Utc>>,
    pub to_timestamp: Option<DateTime<Utc>>,
}

impl EventFilter {
    pub fn matches(&self, event: &RichObserverEvent) -> bool {
        if let Some(ref types) = self.event_types {
            if !types.contains(&event.event_type) {
                return false;
            }
        }

        if let Some(ref components) = self.components {
            if !components.iter().any(|c| event.component.contains(c)) {
                return false;
            }
        }

        if let Some(from) = self.from_timestamp {
            if event.timestamp < from {
                return false;
            }
        }

        if let Some(to) = self.to_timestamp {
            if event.timestamp > to {
                return false;
            }
        }

        true
    }

    pub fn for_type(event_type: ObserverEventType) -> Self {
        Self {
            event_types: Some(vec![event_type]),
            ..Default::default()
        }
    }

    pub fn for_component(component: &str) -> Self {
        Self {
            components: Some(vec![component.to_string()]),
            ..Default::default()
        }
    }
}

impl Observer for PersistentObserver {
    fn name(&self) -> &str {
        "persistent"
    }

    fn record_correlated_event(&self, event: &ObserverEvent, correlation: TelemetryCorrelation) {
        let rich_event = event.to_rich_event_auto_with_correlation(correlation);
        if let Err(e) = self.ensure_dir() {
            tracing::warn!("failed to create observability directory: {}", e);
            return;
        }
        if let Err(e) = self.rotate_if_needed() {
            tracing::warn!("failed to rotate observability file: {}", e);
        }

        let json = match serde_json::to_string(&rich_event) {
            Ok(j) => j,
            Err(e) => {
                tracing::warn!("failed to serialize observer event: {}", e);
                return;
            }
        };

        let mut file = match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.storage_path)
        {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("failed to open observability file: {}", e);
                return;
            }
        };

        use std::io::Write;
        if let Err(e) = writeln!(file, "{}", json) {
            tracing::warn!("failed to write observability event: {}", e);
        }
    }

    fn clone_box(&self) -> Box<dyn Observer> {
        Box::new(Self::from_storage_path(
            self.storage_path.clone(),
            self.max_events,
        ))
    }
}

pub struct JsonlObserver {
    storage_path: PathBuf,
}

impl JsonlObserver {
    pub fn new(workspace_dir: Option<&Path>) -> Self {
        let storage_path = workspace_dir
            .map(|p| p.join("observability").join("events.jsonl"))
            .unwrap_or_else(|| PathBuf::from("maestroclaw-events.jsonl"));
        Self { storage_path }
    }

    fn ensure_dir(&self) -> std::io::Result<()> {
        if let Some(parent) = self.storage_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        Ok(())
    }
}

impl Observer for JsonlObserver {
    fn name(&self) -> &str {
        "jsonl"
    }

    fn record_correlated_event(&self, event: &ObserverEvent, correlation: TelemetryCorrelation) {
        let rich_event = event.to_rich_event_auto_with_correlation(correlation);
        if let Err(e) = self.ensure_dir() {
            tracing::warn!("failed to create observability directory: {}", e);
            return;
        }

        let json = match serde_json::to_string(&rich_event) {
            Ok(j) => j,
            Err(e) => {
                tracing::warn!("failed to serialize observer event: {}", e);
                return;
            }
        };

        let mut file = match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.storage_path)
        {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("failed to open jsonl file: {}", e);
                return;
            }
        };

        use std::io::Write;
        if let Err(e) = writeln!(file, "{}", json) {
            tracing::warn!("failed to write jsonl event: {}", e);
        }
    }

    fn clone_box(&self) -> Box<dyn Observer> {
        Box::new(Self {
            storage_path: self.storage_path.clone(),
        })
    }
}

pub fn create_observer(backend: &str, workspace_dir: Option<&Path>) -> Box<dyn Observer> {
    match backend {
        "log" => Box::new(LogObserver),
        "jsonl" => Box::new(JsonlObserver::new(workspace_dir)),
        "persistent" | "file" => {
            if let Some(dir) = workspace_dir {
                Box::new(PersistentObserver::new(dir))
            } else {
                Box::new(NoopObserver)
            }
        }
        _ => Box::new(NoopObserver),
    }
}

pub struct MultiObserver {
    observers: Vec<Box<dyn Observer>>,
}

impl MultiObserver {
    pub fn new() -> Self {
        Self {
            observers: Vec::new(),
        }
    }

    pub fn add_observer(&mut self, observer: Box<dyn Observer>) {
        self.observers.push(observer);
    }

    pub fn broadcast(&self, event: &ObserverEvent) {
        for observer in &self.observers {
            observer.record_event(event);
        }
    }

    pub fn broadcast_correlated(&self, event: &ObserverEvent, correlation: TelemetryCorrelation) {
        for observer in &self.observers {
            observer.record_correlated_event(event, correlation.clone());
        }
    }
}

impl Default for MultiObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl Observer for MultiObserver {
    fn name(&self) -> &str {
        "multi"
    }

    fn record_correlated_event(&self, event: &ObserverEvent, correlation: TelemetryCorrelation) {
        self.broadcast_correlated(event, correlation);
    }

    fn clone_box(&self) -> Box<dyn Observer> {
        let mut multi = MultiObserver::new();
        for obs in &self.observers {
            multi.add_observer(obs.clone_box());
        }
        Box::new(multi)
    }
}

fn sort_breakdown(map: std::collections::HashMap<String, usize>) -> Vec<EventCountBreakdown> {
    let mut values: Vec<EventCountBreakdown> = map
        .into_iter()
        .map(|(key, count)| EventCountBreakdown { key, count })
        .collect();
    values.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
    values
}

fn field_matches(actual: &Option<String>, expected: &Option<String>) -> bool {
    expected
        .as_ref()
        .map(|value| actual.as_ref() == Some(value))
        .unwrap_or(true)
}

fn value_matches<T: PartialEq + Copy>(actual: Option<T>, expected: Option<T>) -> bool {
    expected.map(|value| actual == Some(value)).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_observer_keeps_name() {
        let observer = NoopObserver;
        observer.record_event(&ObserverEvent::SchedulerTick);
        assert_eq!(observer.name(), "noop");
    }

    #[test]
    fn log_observer_keeps_name() {
        let observer = LogObserver;
        observer.record_event(&ObserverEvent::AgentStart {
            tool: "claude".into(),
        });
        assert_eq!(observer.name(), "log");
    }

    #[test]
    fn unknown_backend_falls_back_to_noop() {
        assert_eq!(create_observer("unknown", None).name(), "noop");
    }

    #[test]
    fn persistent_observer_uses_workspace_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let observer = create_observer("persistent", Some(tmp.path()));
        assert_eq!(observer.name(), "persistent");
    }

    #[test]
    fn rich_event_converts_agent_start() {
        let event = ObserverEvent::AgentStart {
            tool: "claude".into(),
        };
        let rich = event.to_rich_event("test-component");
        assert_eq!(rich.event_type, ObserverEventType::AgentStart);
        assert_eq!(rich.component, "test-component");
        assert_eq!(rich.metadata.tool, Some("claude".into()));
    }

    #[test]
    fn rich_event_converts_agent_error() {
        let event = ObserverEvent::AgentError {
            tool: "claude".into(),
            error: "timeout".into(),
        };
        let rich = event.to_rich_event("test-component");
        assert_eq!(rich.event_type, ObserverEventType::AgentError);
        assert_eq!(rich.metadata.error, Some("timeout".into()));
        assert_eq!(rich.metadata.success, Some(false));
    }

    #[test]
    fn rich_event_converts_scheduler_job() {
        let event = ObserverEvent::SchedulerJobComplete {
            job_id: "job-123".into(),
            job_type: "shell".into(),
            duration_ms: 5000,
            success: true,
        };
        let rich = event.to_rich_event("scheduler");
        assert_eq!(rich.event_type, ObserverEventType::SchedulerJobComplete);
        assert_eq!(rich.metadata.job_id, Some("job-123".into()));
        assert_eq!(rich.metadata.duration_ms, Some(5000));
        assert_eq!(rich.metadata.success, Some(true));
    }

    #[test]
    fn rich_event_converts_heartbeat_task() {
        let event = ObserverEvent::HeartbeatTaskComplete {
            task_name: "Check updates".into(),
            duration_ms: 3000,
            success: true,
        };
        let rich = event.to_rich_event("heartbeat");
        assert_eq!(rich.event_type, ObserverEventType::HeartbeatTaskComplete);
        assert_eq!(rich.metadata.task_name, Some("Check updates".into()));
    }

    #[test]
    fn rich_event_auto_infers_component() {
        let event = ObserverEvent::ChannelMessage {
            channel: "slack".into(),
            sender: "user-1".into(),
            message_id: "msg-1".into(),
        };

        let rich = event.to_rich_event_auto();

        assert_eq!(rich.component, "channel:slack");
        assert_eq!(rich.metadata.channel, Some("slack".into()));
    }

    #[test]
    fn multi_observer_records_to_all() {
        let multi = MultiObserver::new();
        multi.broadcast(&ObserverEvent::DaemonStart);
    }

    #[test]
    fn jsonl_observer_creates_default_file() {
        let observer = JsonlObserver::new(None);
        assert_eq!(observer.name(), "jsonl");
        assert_eq!(
            observer.storage_path,
            PathBuf::from("maestroclaw-events.jsonl")
        );
    }

    #[test]
    fn jsonl_observer_writes_to_specified_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let observer = JsonlObserver::new(Some(tmp.path()));
        observer.record_event(&ObserverEvent::DaemonStart);
        let expected = tmp.path().join("observability").join("events.jsonl");
        assert!(expected.exists());
    }

    #[test]
    fn jsonl_backend_creates_jsonl_observer() {
        let observer = create_observer("jsonl", None);
        assert_eq!(observer.name(), "jsonl");
    }

    #[test]
    fn jsonl_backend_with_workspace_uses_workspace() {
        let tmp = tempfile::TempDir::new().unwrap();
        let observer = create_observer("jsonl", Some(tmp.path()));
        observer.record_event(&ObserverEvent::DaemonStart);
        assert_eq!(observer.name(), "jsonl");
        assert!(tmp
            .path()
            .join("observability")
            .join("events.jsonl")
            .exists());
    }
}
