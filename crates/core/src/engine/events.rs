//! Ordered Event Stream
//!
//! This module implements Moltis-style ordered event streaming:
//! - Event types: thinking, tool-start, tool-end, delta, retry, final, error
//! - Event ordering with sequence numbers
//! - Size-capped payloads for persistence safety
//!
//! Based on Moltis patterns from `analysis_foundation_20260217.md`:
//! - `crates/gateway/src/chat.rs:run_streaming`
//! - Event set: thinking, deltas, tool start/end, retries, sub-agent lifecycle

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Global sequence counter for event ordering.
static GLOBAL_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Kind of event in the stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    /// Agent is thinking (reasoning step).
    Thinking,
    /// Tool execution started.
    ToolStart,
    /// Tool execution completed.
    ToolEnd,
    /// Text delta (streaming output).
    Delta,
    /// Retry attempt (after failure).
    Retry,
    /// Final response (terminal).
    Final,
    /// Error occurred.
    Error,
    /// Sub-agent started (delegation).
    SubAgentStart,
    /// Sub-agent completed (delegation).
    SubAgentEnd,
}

impl EventKind {
    /// Get the logical ordering value for this event kind.
    pub fn order(&self) -> u8 {
        match self {
            Self::Thinking => 0,
            Self::ToolStart => 1,
            Self::ToolEnd => 2,
            Self::Delta => 3,
            Self::Retry => 4,
            Self::SubAgentStart => 5,
            Self::SubAgentEnd => 6,
            Self::Error => 7,
            Self::Final => 8,
        }
    }

    /// Check if this is a terminal event (ends the stream).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Final | Self::Error)
    }

    /// Check if this is a sub-agent event.
    pub fn is_sub_agent(&self) -> bool {
        matches!(self, Self::SubAgentStart | Self::SubAgentEnd)
    }
}

impl std::fmt::Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Thinking => write!(f, "thinking"),
            Self::ToolStart => write!(f, "tool_start"),
            Self::ToolEnd => write!(f, "tool_end"),
            Self::Delta => write!(f, "delta"),
            Self::Retry => write!(f, "retry"),
            Self::SubAgentStart => write!(f, "sub_agent_start"),
            Self::SubAgentEnd => write!(f, "sub_agent_end"),
            Self::Final => write!(f, "final"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// Configuration for payload size capping.
#[derive(Debug, Clone)]
pub struct SizeConfig {
    max_size: usize,
}

impl SizeConfig {
    /// Create a new size config with the given max size in bytes.
    pub fn new(max_size: usize) -> Self {
        Self { max_size }
    }

    /// Get the maximum allowed size.
    pub fn max_size(&self) -> usize {
        self.max_size
    }
}

impl Default for SizeConfig {
    fn default() -> Self {
        Self {
            // Default 1MB max payload size
            max_size: 1024 * 1024,
        }
    }
}

/// Event payload with optional size capping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventPayload {
    /// Text content.
    Text(String),
    /// JSON content.
    Json(serde_json::Value),
    /// Binary content (base64 encoded).
    Binary(String),
    /// Empty payload.
    Empty,
}

impl EventPayload {
    /// Create a text payload.
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text(content.into())
    }

    /// Create a text payload with size capping.
    pub fn text_with_cap(content: &str, config: &SizeConfig) -> Self {
        if content.len() <= config.max_size {
            Self::Text(content.to_string())
        } else {
            let truncated_len = config.max_size.saturating_sub(20); // Leave room for marker
            let mut truncated = content[..truncated_len].to_string();
            truncated.push_str("... [truncated]");
            Self::Text(truncated)
        }
    }

    /// Create a JSON payload.
    pub fn json(value: serde_json::Value) -> Self {
        Self::Json(value)
    }

    /// Create a JSON payload with size capping.
    pub fn json_with_cap(value: &serde_json::Value, config: &SizeConfig) -> Self {
        let serialized = serde_json::to_string(value).unwrap_or_default();
        if serialized.len() <= config.max_size {
            Self::Json(value.clone())
        } else {
            // Truncate to a summary
            Self::Json(serde_json::json!({
                "truncated": true,
                "original_size": serialized.len(),
                "message": "Payload too large, truncated"
            }))
        }
    }

    /// Get the payload as text, if it's a text payload.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Get the payload as JSON, if it's a JSON payload.
    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Json(v) => Some(v),
            _ => None,
        }
    }

    /// Check if this is a text payload.
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Check if this is a JSON payload.
    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json(_))
    }

    /// Get the approximate size of this payload in bytes.
    pub fn size(&self) -> usize {
        match self {
            Self::Text(s) => s.len(),
            Self::Json(v) => serde_json::to_string(v).map(|s| s.len()).unwrap_or(0),
            Self::Binary(s) => s.len(),
            Self::Empty => 0,
        }
    }
}

/// An event in the ordered stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Sequence number for ordering.
    sequence: u64,
    /// Unix timestamp in milliseconds.
    timestamp: u64,
    /// Event kind.
    kind: EventKind,
    /// Event content/message.
    content: String,
    /// Optional payload.
    payload: Option<EventPayload>,
    /// Tool name (for tool events).
    tool_name: Option<String>,
    /// Tool request ID (for tool events).
    tool_request_id: Option<String>,
    /// Error code (for error events).
    error_code: Option<String>,
    /// Whether error is recoverable.
    is_recoverable: bool,
}

impl Event {
    /// Create a new event with auto-generated sequence and timestamp.
    pub fn new(kind: EventKind, content: impl Into<String>) -> Self {
        Self {
            sequence: GLOBAL_SEQUENCE.fetch_add(1, AtomicOrdering::SeqCst),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            kind,
            content: content.into(),
            payload: None,
            tool_name: None,
            tool_request_id: None,
            error_code: None,
            is_recoverable: false,
        }
    }

    /// Create a new event with a payload.
    pub fn new_with_payload(kind: EventKind, payload: EventPayload) -> Self {
        let content = match &payload {
            EventPayload::Text(s) => s.clone(),
            EventPayload::Json(v) => v.to_string(),
            EventPayload::Binary(_) => "[binary data]".to_string(),
            EventPayload::Empty => String::new(),
        };
        Self {
            sequence: GLOBAL_SEQUENCE.fetch_add(1, AtomicOrdering::SeqCst),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            kind,
            content,
            payload: Some(payload),
            tool_name: None,
            tool_request_id: None,
            error_code: None,
            is_recoverable: false,
        }
    }

    /// Get the sequence number.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Get the timestamp.
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Get the event kind.
    pub fn kind(&self) -> &EventKind {
        &self.kind
    }

    /// Get the content/message.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the payload, if any.
    pub fn payload(&self) -> Option<&EventPayload> {
        self.payload.as_ref()
    }

    /// Set the tool name (for tool events).
    pub fn set_tool_name(&mut self, name: impl Into<String>) {
        self.tool_name = Some(name.into());
    }

    /// Get the tool name.
    pub fn tool_name(&self) -> Option<&str> {
        self.tool_name.as_deref()
    }

    /// Set the tool request ID.
    pub fn set_tool_request_id(&mut self, id: impl Into<String>) {
        self.tool_request_id = Some(id.into());
    }

    /// Get the tool request ID.
    pub fn tool_request_id(&self) -> Option<&str> {
        self.tool_request_id.as_deref()
    }

    /// Set the error code.
    pub fn set_error_code(&mut self, code: impl Into<String>) {
        self.error_code = Some(code.into());
    }

    /// Get the error code.
    pub fn error_code(&self) -> Option<&str> {
        self.error_code.as_deref()
    }

    /// Set whether the error is recoverable.
    pub fn set_is_recoverable(&mut self, recoverable: bool) {
        self.is_recoverable = recoverable;
    }

    /// Check if the error is recoverable.
    pub fn is_recoverable(&self) -> bool {
        self.is_recoverable
    }
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} (seq={}): {}",
            self.timestamp, self.kind, self.sequence, self.content
        )
    }
}

/// A buffer for collecting and managing events.
#[derive(Debug, Clone)]
pub struct EventBuffer {
    events: Arc<RwLock<Vec<Event>>>,
}

impl EventBuffer {
    /// Create a new empty event buffer.
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Push an event to the buffer.
    pub fn push(&mut self, event: Event) -> Event {
        let mut events = self.events.write().unwrap();
        events.push(event);
        // Return a reference-like clone with the sequence
        events.last().cloned().unwrap()
    }

    /// Get the number of events in the buffer.
    pub fn len(&self) -> usize {
        let events = self.events.read().unwrap();
        events.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all events from the buffer.
    pub fn clear(&mut self) {
        let mut events = self.events.write().unwrap();
        events.clear();
    }

    /// Get the last event, if any.
    pub fn last(&self) -> Option<Event> {
        let events = self.events.read().unwrap();
        events.last().cloned()
    }

    /// Iterate over all events.
    pub fn iter(&self) -> impl Iterator<Item = Event> {
        let events = self.events.read().unwrap();
        events.clone().into_iter()
    }

    /// Filter events by kind.
    pub fn filter_by_kind(&self, kind: EventKind) -> impl Iterator<Item = Event> {
        let events = self.events.read().unwrap();
        events.clone().into_iter().filter(move |e| e.kind == kind)
    }

    /// Get all events as a vector.
    pub fn to_vec(&self) -> Vec<Event> {
        let events = self.events.read().unwrap();
        events.clone()
    }
}

impl Default for EventBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_kind_ordering() {
        assert!(EventKind::Thinking.order() < EventKind::Final.order());
    }

    #[test]
    fn test_event_new() {
        let event = Event::new(EventKind::Thinking, "Test");
        assert_eq!(event.kind(), &EventKind::Thinking);
        assert!(event.sequence() > 0);
    }

    #[test]
    fn test_event_buffer() {
        let mut buffer = EventBuffer::new();
        assert!(buffer.is_empty());

        buffer.push(Event::new(EventKind::Thinking, "Test"));
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_payload_size_capping() {
        let large_text = "x".repeat(100_000);
        let config = SizeConfig::new(1000);
        let payload = EventPayload::text_with_cap(&large_text, &config);
        assert!(payload.size() <= 1020); // Allow for truncation marker
    }
}
