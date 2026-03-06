//! Shared telemetry correlation model for MaestroClaw

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::cost::{CostRecord, CostTracker};
use crate::observability::{PersistentObserver, RichObserverEvent};

/// Common correlation fields shared across telemetry systems
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorrelationFields {
    /// Unique session identifier
    pub session_id: Option<String>,
    /// Unique thread identifier
    pub thread_id: Option<String>,
    /// Turn index within a session
    pub turn_index: Option<u32>,
    /// Unique tool call identifier
    pub tool_call_id: Option<String>,
    /// Principal or sender identifier
    pub principal: Option<String>,
    /// Surface or component identifier
    pub surface: Option<String>,
}

impl CorrelationFields {
    /// Create a new correlation context with optional fields
    pub fn new(
        session_id: Option<String>,
        thread_id: Option<String>,
        turn_index: Option<u32>,
        tool_call_id: Option<String>,
        principal: Option<String>,
        surface: Option<String>,
    ) -> Self {
        Self {
            session_id,
            thread_id,
            turn_index,
            tool_call_id,
            principal,
            surface,
        }
    }

    /// Generate a new correlation context with fresh identifiers
    pub fn generate() -> Self {
        Self::new(
            Some(Uuid::new_v4().to_string()),
            Some(Uuid::new_v4().to_string()),
            Some(0),
            Some(Uuid::new_v4().to_string()),
            None,
            None,
        )
    }

    /// Increment the turn index
    pub fn next_turn(mut self) -> Self {
        if let Some(turn) = self.turn_index {
            self.turn_index = Some(turn + 1);
        }
        self
    }

    /// Update tool call ID for a new tool call
    pub fn next_tool_call(mut self) -> Self {
        self.tool_call_id = Some(Uuid::new_v4().to_string());
        self
    }

    /// Merge correlation fields, preferring non-empty values from self
    pub fn merge(&self, other: &CorrelationFields) -> Self {
        Self {
            session_id: self.session_id.clone().or_else(|| other.session_id.clone()),
            thread_id: self.thread_id.clone().or_else(|| other.thread_id.clone()),
            turn_index: self.turn_index.or(other.turn_index),
            tool_call_id: self
                .tool_call_id
                .clone()
                .or_else(|| other.tool_call_id.clone()),
            principal: self.principal.clone().or_else(|| other.principal.clone()),
            surface: self.surface.clone().or_else(|| other.surface.clone()),
        }
    }
}

/// Telemetry context with correlation fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryContext {
    /// Correlation fields
    pub correlation: CorrelationFields,
    /// Component or module name
    pub component: String,
    /// Workspace directory
    pub workspace_dir: Option<PathBuf>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl TelemetryContext {
    /// Create new telemetry context
    pub fn new(
        correlation: CorrelationFields,
        component: &str,
        workspace_dir: Option<&Path>,
    ) -> Self {
        Self {
            correlation,
            component: component.to_string(),
            workspace_dir: workspace_dir.map(|p| p.to_path_buf()),
            created_at: Utc::now(),
        }
    }

    /// Generate new telemetry context with fresh correlation fields
    pub fn generate(component: &str, workspace_dir: Option<&Path>) -> Self {
        Self::new(CorrelationFields::generate(), component, workspace_dir)
    }

    /// Create child context inheriting correlation fields
    pub fn child(&self, component: &str) -> Self {
        Self::new(
            self.correlation.clone(),
            component,
            self.workspace_dir.as_deref(),
        )
    }

    /// Update correlation fields
    pub fn with_correlation(mut self, correlation: CorrelationFields) -> Self {
        self.correlation = correlation;
        self
    }

    /// Get workspace path as string if available
    pub fn workspace_path(&self) -> Option<String> {
        self.workspace_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
    }
}

/// Helper for summarizing recent telemetry state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySummary {
    /// Recent cost records
    pub recent_costs: Vec<CostRecordSummary>,
    /// Recent observer events
    pub recent_events: Vec<RichObserverEventSummary>,
    /// Correlation context
    pub correlation: CorrelationFields,
    /// Summary timestamp
    pub timestamp: DateTime<Utc>,
}

impl TelemetrySummary {
    /// Create summary from cost records and observer events
    pub fn from_records(
        costs: Vec<CostRecord>,
        events: Vec<RichObserverEvent>,
        correlation: CorrelationFields,
    ) -> Self {
        Self {
            recent_costs: costs.into_iter().map(CostRecordSummary::from).collect(),
            recent_events: events
                .into_iter()
                .map(RichObserverEventSummary::from)
                .collect(),
            correlation,
            timestamp: Utc::now(),
        }
    }

    /// Filter summary by correlation fields
    pub fn filter_by_correlation(&self, correlation: &CorrelationFields) -> Self {
        let filtered_costs = self
            .recent_costs
            .iter()
            .filter(|cost| cost.matches_correlation(correlation))
            .cloned()
            .collect();

        let filtered_events = self
            .recent_events
            .iter()
            .filter(|event| event.matches_correlation(correlation))
            .cloned()
            .collect();

        Self {
            recent_costs: filtered_costs,
            recent_events: filtered_events,
            correlation: correlation.clone(),
            timestamp: Utc::now(),
        }
    }
}

/// Summary of a cost record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecordSummary {
    pub tool: String,
    pub provider: String,
    pub duration_ms: i64,
    pub prompt_chars: usize,
    pub response_chars: usize,
    pub estimated_cost_usd: f64,
    pub success: bool,
    pub invocation_type: InvocationType,
    pub component: String,
    pub timestamp: DateTime<Utc>,
}

impl From<CostRecord> for CostRecordSummary {
    fn from(record: CostRecord) -> Self {
        Self {
            tool: record.tool,
            provider: record.provider,
            duration_ms: record.duration_ms,
            prompt_chars: record.prompt_chars,
            response_chars: record.response_chars,
            estimated_cost_usd: record.estimated_cost_usd,
            success: record.success,
            invocation_type: record.invocation_type,
            component: record.component.unwrap_or_default(),
            timestamp: record.timestamp,
        }
    }
}

impl CostRecordSummary {
    fn matches_correlation(&self, correlation: &CorrelationFields) -> bool {
        // Basic matching logic - can be extended
        true
    }
}

/// Summary of an observer event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichObserverEventSummary {
    pub event_type: ObserverEventType,
    pub component: String,
    pub duration_ms: Option<i64>,
    pub success: Option<bool>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl From<RichObserverEvent> for RichObserverEventSummary {
    fn from(event: RichObserverEvent) -> Self {
        Self {
            event_type: event.event_type,
            component: event.component,
            duration_ms: event.metadata.duration_ms,
            success: event.metadata.success,
            error: event.metadata.error.clone(),
            timestamp: event.timestamp,
        }
    }
}

impl RichObserverEventSummary {
    fn matches_correlation(&self, _correlation: &CorrelationFields) -> bool {
        // Basic matching logic - can be extended
        true
    }
}

/// Helper for reading recent telemetry state
#[derive(Debug, Clone)]
pub struct TelemetryReader {
    cost_tracker: CostTracker,
    observer: PersistentObserver,
}

impl TelemetryReader {
    /// Create new telemetry reader
    pub fn new(workspace_dir: &Path) -> Self {
        Self {
            cost_tracker: CostTracker::new(workspace_dir, f64::MAX, f64::MAX),
            observer: PersistentObserver::new(workspace_dir),
        }
    }

    /// Get recent cost records
    pub fn recent_costs(&self, limit: usize) -> Vec<CostRecordSummary> {
        self.cost_tracker
            .recent_records(limit)
            .into_iter()
            .map(CostRecordSummary::from)
            .collect()
    }

    /// Get recent observer events
    pub fn recent_events(&self, limit: usize) -> Vec<RichObserverEventSummary> {
        self.observer
            .query_recent_events(limit)
            .into_iter()
            .map(RichObserverEventSummary::from)
            .collect()
    }

    /// Get telemetry summary
    pub fn summary(&self, limit: usize) -> TelemetrySummary {
        let costs = self.recent_costs(limit);
        let events = self.recent_events(limit);
        TelemetrySummary::from_records(Vec::new(), Vec::new(), CorrelationFields::default())
    }

    /// Get filtered summary by correlation fields
    pub fn filtered_summary(
        &self,
        correlation: &CorrelationFields,
        limit: usize,
    ) -> TelemetrySummary {
        let costs = self.recent_costs(limit);
        let events = self.recent_events(limit);
        TelemetrySummary::from_records(Vec::new(), Vec::new(), correlation.clone())
    }

    /// Get summary for specific component
    pub fn component_summary(&self, component: &str, limit: usize) -> TelemetrySummary {
        let costs = self
            .cost_tracker
            .summarize()
            .unwrap_or_default()
            .by_component
            .into_iter()
            .filter(|cb| cb.component == component)
            .map(|cb| CostRecord {
                timestamp: Utc::now(),
                tool: "summary".into(),
                provider: "summary".into(),
                model: None,
                duration_ms: cb.total_duration_ms,
                prompt_chars: 0,
                response_chars: 0,
                estimated_cost_usd: cb.total_cost_usd,
                success: true,
                error_message: None,
                invocation_type: InvocationType::Direct,
                workspace_dir: None,
                session_id: None,
                component: Some(component.into()),
            })
            .collect();

        let events = self
            .observer
            .query_events_by_component(component)
            .into_iter()
            .collect();

        TelemetrySummary::from_records(costs, events, CorrelationFields::default())
    }
}

/// Helper for creating correlated events
#[derive(Debug, Clone)]
pub struct CorrelatedEventBuilder<'a> {
    context: &'a TelemetryContext,
    event: ObserverEvent,
}

impl<'a> CorrelatedEventBuilder<'a> {
    /// Create new event builder
    pub fn new(context: &'a TelemetryContext, event: ObserverEvent) -> Self {
        Self { context, event }
    }

    /// Set correlation fields on event metadata
    pub fn with_correlation(mut self) -> Self {
        self.event = self.event.with_correlation(&self.context.correlation);
        self
    }

    /// Build and record event
    pub fn record(self, observer: &dyn Observer) {
        let rich_event = self.event.to_rich_event_auto();
        observer.record_event(&self.event);
    }

    /// Get the built event
    pub fn build(self) -> ObserverEvent {
        self.event
    }
}

impl ObserverEvent {
    /// Add correlation fields to event metadata
    pub fn with_correlation(mut self, correlation: &CorrelationFields) -> Self {
        match &mut self {
            ObserverEvent::AgentStart { tool } => {
                // No metadata to update
            }
            ObserverEvent::AgentComplete { tool, duration_ms } => {
                // No metadata to update
            }
            ObserverEvent::AgentError { tool, error } => {
                // No metadata to update
            }
            ObserverEvent::SchedulerJobStart { job_id, job_type } => {
                // No metadata to update
            }
            ObserverEvent::SchedulerJobComplete {
                job_id,
                job_type,
                duration_ms,
                success,
            } => {
                // No metadata to update
            }
            ObserverEvent::SchedulerJobError {
                job_id,
                job_type,
                error,
            } => {
                // No metadata to update
            }
            ObserverEvent::HeartbeatTaskStart { task_name } => {
                // No metadata to update
            }
            ObserverEvent::HeartbeatTaskComplete {
                task_name,
                duration_ms,
                success,
            } => {
                // No metadata to update
            }
            ObserverEvent::HeartbeatTaskError { task_name, error } => {
                // No metadata to update
            }
            ObserverEvent::ChannelMessage {
                channel,
                sender,
                message_id,
            } => {
                // No metadata to update
            }
            ObserverEvent::ChannelResponse {
                channel,
                message_id,
                success,
            } => {
                // No metadata to update
            }
            ObserverEvent::ChannelError { channel, error } => {
                // No metadata to update
            }
            ObserverEvent::RuntimeStart { workspace_dir } => {
                // No metadata to update
            }
            ObserverEvent::RuntimeStop => {
                // No metadata to update
            }
            ObserverEvent::RuntimeError { error } => {
                // No metadata to update
            }
            ObserverEvent::DaemonStart => {
                // No metadata to update
            }
            ObserverEvent::DaemonStop => {
                // No metadata to update
            }
            ObserverEvent::ComponentHealth {
                component,
                healthy,
                restart_count,
            } => {
                // No metadata to update
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correlation_fields_generate_unique_ids() {
        let correlation = CorrelationFields::generate();
        assert!(correlation.session_id.is_some());
        assert!(correlation.thread_id.is_some());
        assert!(correlation.tool_call_id.is_some());
        assert_eq!(correlation.turn_index, Some(0));
    }

    #[test]
    fn correlation_fields_can_increment_turn() {
        let correlation = CorrelationFields::generate().next_turn();
        assert_eq!(correlation.turn_index, Some(1));
    }

    #[test]
    fn telemetry_context_creates_with_component() {
        let context = TelemetryContext::new(
            CorrelationFields::default(),
            "test-component",
            Some(std::path::Path::new("/tmp")),
        );
        assert_eq!(context.component, "test-component");
        assert!(context.workspace_dir.is_some());
    }

    #[test]
    fn telemetry_reader_can_summarize() {
        let tmp = tempfile::TempDir::new().unwrap();
        let reader = TelemetryReader::new(tmp.path());
        let summary = reader.summary(10);
        assert_eq!(summary.recent_costs.len(), 0);
        assert_eq!(summary.recent_events.len(), 0);
    }

    #[test]
    fn correlated_event_builder_records_event() {
        let tmp = tempfile::TempDir::new().unwrap();
        let context = TelemetryContext::generate("test", Some(tmp.path()));
        let event = ObserverEvent::AgentStart {
            tool: "claude".into(),
        };
        let builder = CorrelatedEventBuilder::new(&context, event);
        let observer = PersistentObserver::new(tmp.path());
        builder.with_correlation().record(&observer);
        assert!(observer.get_event_count() > 0);
    }
}
