//! Persistence Pipeline for Session/Turn/Tool Events
//!
//! This module implements persistence patterns inspired by IronClaw/Moltis:
//! - Session/Turn/Tool event storage
//! - Reasoning breadcrumbs persistence
//! - Media/large payload references as lightweight pointers
//!
//! Key patterns from reference codebases:
//! - IronClaw: `src/agent/session.rs` - Session/Thread/Turn model
//! - Moltis: `crates/gateway/src/message_log_store.rs` - SQLite message persistence
//! - IronClaw: `src/db/libsql_backend.rs` - SQLite persistence patterns

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use uuid::Uuid;

use super::events::Event;

/// Maximum size for inline payload storage (10KB)
const MAX_INLINE_PAYLOAD_SIZE: usize = 10 * 1024;

// ==================== Errors ====================

/// Errors that can occur during persistence operations.
#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("{0} with id '{1}' not found")]
    NotFound(String, String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Lock error: {0}")]
    Lock(String),
}

// ==================== Turn State ====================

/// State of a turn in the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnState {
    /// Turn is being processed.
    Processing,
    /// Turn completed successfully.
    Completed,
    /// Turn failed with an error.
    Failed,
    /// Turn was interrupted.
    Interrupted,
}

// ==================== Media Reference ====================

/// Reference to external media/large payload storage.
///
/// Instead of storing large payloads inline, we store a lightweight pointer
/// to the external storage location. This keeps the database efficient.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaRef {
    /// URI pointing to the media content (e.g., "media://images/screenshot.png")
    uri: String,
    /// MIME type of the content
    content_type: String,
    /// Size in bytes
    size_bytes: u64,
    /// Optional checksum for integrity verification
    #[serde(skip_serializing_if = "Option::is_none")]
    checksum: Option<String>,
    /// When the media was stored
    created_at: chrono::DateTime<chrono::Utc>,
}

impl MediaRef {
    /// Create a new media reference.
    pub fn new(uri: impl Into<String>, content_type: impl Into<String>, size_bytes: u64) -> Self {
        Self {
            uri: uri.into(),
            content_type: content_type.into(),
            size_bytes,
            checksum: None,
            created_at: chrono::Utc::now(),
        }
    }

    /// Create a media reference with a checksum.
    pub fn with_checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    /// Get the URI.
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Get the content type.
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Get the size in bytes.
    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    /// Get the checksum, if any.
    pub fn checksum(&self) -> Option<&str> {
        self.checksum.as_deref()
    }
}

// ==================== Reasoning Breadcrumb ====================

/// A reasoning breadcrumb captured during agent thinking.
///
/// Stores intermediate reasoning steps for later analysis and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningBreadcrumb {
    /// Unique ID for this breadcrumb
    id: Uuid,
    /// Turn this breadcrumb belongs to
    turn_id: Uuid,
    /// The reasoning content
    content: String,
    /// Sequence number for ordering
    sequence: i32,
    /// When this was created
    created_at: chrono::DateTime<chrono::Utc>,
}

impl ReasoningBreadcrumb {
    /// Create a new reasoning breadcrumb.
    pub fn new(turn_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            turn_id,
            content: content.into(),
            sequence: 0,
            created_at: chrono::Utc::now(),
        }
    }

    /// Create a breadcrumb with a specific sequence number.
    pub fn with_sequence(mut self, sequence: i32) -> Self {
        self.sequence = sequence;
        self
    }

    /// Get the ID.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the turn ID.
    pub fn turn_id(&self) -> Uuid {
        self.turn_id
    }

    /// Get the content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the sequence number.
    pub fn sequence(&self) -> i32 {
        self.sequence
    }
}

// ==================== Tool Event Record ====================

/// Record of a tool execution event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEventRecord {
    /// Unique ID for this event
    id: Uuid,
    /// Turn this event belongs to
    turn_id: Uuid,
    /// Tool name
    tool_name: String,
    /// Tool input parameters
    input: serde_json::Value,
    /// Tool result (if successful) - stored inline if small
    result: Option<serde_json::Value>,
    /// Tool error (if failed)
    error: Option<String>,
    /// Reference to external result storage (for large payloads)
    result_media_ref: Option<MediaRef>,
    /// Request ID from the LLM
    request_id: Option<String>,
    /// Execution duration in milliseconds
    duration_ms: Option<u64>,
    /// When this event was created
    created_at: chrono::DateTime<chrono::Utc>,
}

impl ToolEventRecord {
    /// Create a new tool event record.
    pub fn new(turn_id: Uuid, tool_name: impl Into<String>, input: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            turn_id,
            tool_name: tool_name.into(),
            input,
            result: None,
            error: None,
            result_media_ref: None,
            request_id: None,
            duration_ms: None,
            created_at: chrono::Utc::now(),
        }
    }

    /// Create with a specific ID (for hydration from DB).
    pub fn with_id(
        id: Uuid,
        turn_id: Uuid,
        tool_name: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        Self {
            id,
            turn_id,
            tool_name: tool_name.into(),
            input,
            result: None,
            error: None,
            result_media_ref: None,
            request_id: None,
            duration_ms: None,
            created_at: chrono::Utc::now(),
        }
    }

    /// Set the request ID.
    pub fn set_request_id(&mut self, request_id: impl Into<String>) {
        self.request_id = Some(request_id.into());
    }

    /// Set the tool result.
    pub fn set_result(&mut self, result: serde_json::Value) {
        self.result = Some(result);
    }

    /// Set the tool error.
    pub fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }

    /// Set the result as a media reference (for large payloads).
    pub fn set_result_media_ref(&mut self, media_ref: MediaRef) {
        self.result_media_ref = Some(media_ref);
    }

    /// Set the execution duration.
    pub fn set_duration_ms(&mut self, duration_ms: u64) {
        self.duration_ms = Some(duration_ms);
    }

    /// Check if a result should be externalized to media storage.
    pub fn should_externalize_result(&self, result: &serde_json::Value) -> bool {
        match serde_json::to_string(result) {
            Ok(json) => json.len() > MAX_INLINE_PAYLOAD_SIZE,
            Err(_) => false,
        }
    }

    /// Get the ID.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the turn ID.
    pub fn turn_id(&self) -> Uuid {
        self.turn_id
    }

    /// Get the tool name.
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    /// Get the input.
    pub fn input(&self) -> &serde_json::Value {
        &self.input
    }

    /// Get the result.
    pub fn result(&self) -> Option<&serde_json::Value> {
        self.result.as_ref()
    }

    /// Get the error.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Get the result media reference.
    pub fn result_media_ref(&self) -> Option<&MediaRef> {
        self.result_media_ref.as_ref()
    }

    /// Get the request ID.
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    /// Get the duration in milliseconds.
    pub fn duration_ms(&self) -> Option<u64> {
        self.duration_ms
    }
}

// ==================== Turn Record ====================

/// A single turn (request/response pair) in a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnRecord {
    /// Unique ID for this turn
    id: Uuid,
    /// Session this turn belongs to
    session_id: Uuid,
    /// Turn number (0-indexed)
    turn_number: u32,
    /// User input
    user_input: String,
    /// Assistant response
    response: Option<String>,
    /// Current state
    state: TurnState,
    /// Error message (if failed)
    error: Option<String>,
    /// Tool events during this turn
    tool_events: Vec<ToolEventRecord>,
    /// Reasoning breadcrumbs
    reasoning_breadcrumbs: Vec<ReasoningBreadcrumb>,
    /// When this turn started
    started_at: chrono::DateTime<chrono::Utc>,
    /// When this turn completed
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl TurnRecord {
    /// Create a new turn record.
    pub fn new(session_id: Uuid, turn_number: u32, user_input: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            turn_number,
            user_input: user_input.into(),
            response: None,
            state: TurnState::Processing,
            error: None,
            tool_events: Vec::new(),
            reasoning_breadcrumbs: Vec::new(),
            started_at: chrono::Utc::now(),
            completed_at: None,
        }
    }

    /// Create with a specific ID (for hydration from DB).
    pub fn with_id(
        id: Uuid,
        session_id: Uuid,
        turn_number: u32,
        user_input: impl Into<String>,
    ) -> Self {
        Self {
            id,
            session_id,
            turn_number,
            user_input: user_input.into(),
            response: None,
            state: TurnState::Processing,
            error: None,
            tool_events: Vec::new(),
            reasoning_breadcrumbs: Vec::new(),
            started_at: chrono::Utc::now(),
            completed_at: None,
        }
    }

    /// Complete this turn with a response.
    pub fn complete(&mut self, response: impl Into<String>) {
        self.response = Some(response.into());
        self.state = TurnState::Completed;
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Fail this turn with an error.
    pub fn fail(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
        self.state = TurnState::Failed;
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Interrupt this turn.
    pub fn interrupt(&mut self) {
        self.state = TurnState::Interrupted;
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Add a tool event.
    pub fn add_tool_event(&mut self, event: ToolEventRecord) {
        self.tool_events.push(event);
    }

    /// Add a reasoning breadcrumb.
    pub fn add_reasoning(&mut self, reasoning: ReasoningBreadcrumb) {
        self.reasoning_breadcrumbs.push(reasoning);
    }

    /// Get the ID.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the session ID.
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    /// Get the turn number.
    pub fn turn_number(&self) -> u32 {
        self.turn_number
    }

    /// Get the user input.
    pub fn user_input(&self) -> &str {
        &self.user_input
    }

    /// Get the response.
    pub fn response(&self) -> Option<&str> {
        self.response.as_deref()
    }

    /// Get the state.
    pub fn state(&self) -> TurnState {
        self.state
    }

    /// Get the error.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Get the tool events.
    pub fn tool_events(&self) -> &[ToolEventRecord] {
        &self.tool_events
    }

    /// Get the reasoning breadcrumbs.
    pub fn reasoning_breadcrumbs(&self) -> &[ReasoningBreadcrumb] {
        &self.reasoning_breadcrumbs
    }

    /// Get the completed at timestamp.
    pub fn completed_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.completed_at
    }
}

// ==================== Session Record ====================

/// A session containing one or more turns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    /// Unique session ID
    id: Uuid,
    /// User ID that owns this session
    user_id: String,
    /// When the session was created
    created_at: chrono::DateTime<chrono::Utc>,
    /// When the session was last active
    last_active_at: chrono::DateTime<chrono::Utc>,
    /// Session metadata
    metadata: serde_json::Value,
    /// Number of turns (cached for quick access)
    turn_count: u32,
}

impl SessionRecord {
    /// Create a new session record.
    pub fn new(user_id: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id: user_id.into(),
            created_at: now,
            last_active_at: now,
            metadata: serde_json::Value::Null,
            turn_count: 0,
        }
    }

    /// Create with a specific ID (for hydration from DB).
    pub fn with_id(id: Uuid, user_id: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            user_id: user_id.into(),
            created_at: now,
            last_active_at: now,
            metadata: serde_json::Value::Null,
            turn_count: 0,
        }
    }

    /// Set metadata.
    pub fn set_metadata(&mut self, metadata: serde_json::Value) {
        self.metadata = metadata;
    }

    /// Update the last active timestamp.
    pub fn touch(&mut self) {
        self.last_active_at = chrono::Utc::now();
    }

    /// Increment the turn count.
    pub fn increment_turn_count(&mut self) {
        self.turn_count += 1;
    }

    /// Get the ID.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the user ID.
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// Get the turn count.
    pub fn turn_count(&self) -> u32 {
        self.turn_count
    }

    /// Get the metadata.
    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }
}

// ==================== Event Extensions ====================

impl Event {
    /// Convert this event to a reasoning breadcrumb.
    pub fn into_reasoning(self, turn_id: Uuid) -> ReasoningBreadcrumb {
        ReasoningBreadcrumb::new(turn_id, self.content().to_string())
    }

    /// Convert this event to a tool event record.
    pub fn into_tool_event(self, turn_id: Uuid, input: serde_json::Value) -> ToolEventRecord {
        let mut record = ToolEventRecord::new(turn_id, self.tool_name().unwrap_or(""), input);
        if let Some(req_id) = self.tool_request_id() {
            record.set_request_id(req_id);
        }
        record
    }
}

// ==================== Persistence Store ====================

/// In-memory persistence store for testing and development.
///
/// In production, this would be backed by SQLite/Turso.
#[derive(Debug, Clone)]
pub struct PersistenceStore {
    sessions: Arc<RwLock<HashMap<Uuid, SessionRecord>>>,
    turns: Arc<RwLock<HashMap<Uuid, TurnRecord>>>,
    tool_events: Arc<RwLock<HashMap<Uuid, ToolEventRecord>>>,
    reasonings: Arc<RwLock<HashMap<Uuid, ReasoningBreadcrumb>>>,
    // Indexes for efficient queries
    turns_by_session: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,
    tool_events_by_turn: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,
    reasonings_by_turn: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,
}

impl PersistenceStore {
    /// Create a new in-memory persistence store.
    pub fn new_in_memory() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            turns: Arc::new(RwLock::new(HashMap::new())),
            tool_events: Arc::new(RwLock::new(HashMap::new())),
            reasonings: Arc::new(RwLock::new(HashMap::new())),
            turns_by_session: Arc::new(RwLock::new(HashMap::new())),
            tool_events_by_turn: Arc::new(RwLock::new(HashMap::new())),
            reasonings_by_turn: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        match self.sessions.read() {
            Ok(sessions) => sessions.is_empty(),
            Err(_) => false,
        }
    }

    // ==================== Session Operations ====================

    /// Save a session record.
    pub fn save_session(&self, session: SessionRecord) -> Result<(), PersistenceError> {
        let id = session.id;
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        sessions.insert(id, session);
        Ok(())
    }

    /// Get a session by ID.
    pub fn get_session(&self, id: Uuid) -> Result<Option<SessionRecord>, PersistenceError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        let turns_by_session = self
            .turns_by_session
            .read()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;

        let mut session = sessions.get(&id).cloned();
        if let Some(ref mut s) = session {
            // Update turn count from index
            if let Some(turn_ids) = turns_by_session.get(&id) {
                s.turn_count = turn_ids.len() as u32;
            }
        }
        Ok(session)
    }

    /// Delete a session and all its turns.
    pub fn delete_session(&self, id: Uuid) -> Result<bool, PersistenceError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        let mut turns = self
            .turns
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        let mut turns_by_session = self
            .turns_by_session
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;

        let removed = sessions.remove(&id).is_some();

        // Remove all turns for this session
        if let Some(turn_ids) = turns_by_session.remove(&id) {
            for turn_id in turn_ids {
                turns.remove(&turn_id);
            }
        }

        Ok(removed)
    }

    /// List sessions for a user.
    pub fn list_sessions(&self, user_id: &str) -> Result<Vec<SessionRecord>, PersistenceError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        Ok(sessions
            .values()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect())
    }

    // ==================== Turn Operations ====================

    /// Save a turn record.
    pub fn save_turn(&self, turn: TurnRecord) -> Result<(), PersistenceError> {
        let id = turn.id;
        let session_id = turn.session_id;

        let mut turns = self
            .turns
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        let mut turns_by_session = self
            .turns_by_session
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;

        turns.insert(id, turn);

        // Update index
        turns_by_session.entry(session_id).or_default().push(id);

        Ok(())
    }

    /// Get a turn by ID.
    pub fn get_turn(&self, id: Uuid) -> Result<Option<TurnRecord>, PersistenceError> {
        let turns = self
            .turns
            .read()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        Ok(turns.get(&id).cloned())
    }

    /// List turns for a session.
    pub fn list_turns(&self, session_id: Uuid) -> Result<Vec<TurnRecord>, PersistenceError> {
        let turns = self
            .turns
            .read()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        let turns_by_session = self
            .turns_by_session
            .read()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;

        if let Some(turn_ids) = turns_by_session.get(&session_id) {
            let mut result: Vec<TurnRecord> = turn_ids
                .iter()
                .filter_map(|id| turns.get(id).cloned())
                .collect();
            // Sort by turn number
            result.sort_by_key(|t| t.turn_number);
            Ok(result)
        } else {
            Ok(Vec::new())
        }
    }

    /// Update a turn.
    pub fn update_turn(&self, turn: TurnRecord) -> Result<(), PersistenceError> {
        let id = turn.id;
        let mut turns = self
            .turns
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        turns.insert(id, turn);
        Ok(())
    }

    // ==================== Tool Event Operations ====================

    /// Save a tool event.
    pub fn save_tool_event(&self, event: ToolEventRecord) -> Result<(), PersistenceError> {
        let id = event.id;
        let turn_id = event.turn_id;

        let mut tool_events = self
            .tool_events
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        let mut tool_events_by_turn = self
            .tool_events_by_turn
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;

        tool_events.insert(id, event);

        // Update index
        tool_events_by_turn.entry(turn_id).or_default().push(id);

        Ok(())
    }

    /// Get a tool event by ID.
    pub fn get_tool_event(&self, id: Uuid) -> Result<Option<ToolEventRecord>, PersistenceError> {
        let tool_events = self
            .tool_events
            .read()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        Ok(tool_events.get(&id).cloned())
    }

    /// List tool events for a turn.
    pub fn list_tool_events(
        &self,
        turn_id: Uuid,
    ) -> Result<Vec<ToolEventRecord>, PersistenceError> {
        let tool_events = self
            .tool_events
            .read()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        let tool_events_by_turn = self
            .tool_events_by_turn
            .read()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;

        if let Some(event_ids) = tool_events_by_turn.get(&turn_id) {
            Ok(event_ids
                .iter()
                .filter_map(|id| tool_events.get(id).cloned())
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    /// Update a tool event.
    pub fn update_tool_event(&self, event: ToolEventRecord) -> Result<(), PersistenceError> {
        let id = event.id;
        let mut tool_events = self
            .tool_events
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        tool_events.insert(id, event);
        Ok(())
    }

    // ==================== Reasoning Operations ====================

    /// Save a reasoning breadcrumb.
    pub fn save_reasoning(&self, reasoning: ReasoningBreadcrumb) -> Result<(), PersistenceError> {
        let id = reasoning.id;
        let turn_id = reasoning.turn_id;

        let mut reasonings = self
            .reasonings
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        let mut reasonings_by_turn = self
            .reasonings_by_turn
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;

        reasonings.insert(id, reasoning);

        // Update index
        reasonings_by_turn.entry(turn_id).or_default().push(id);

        Ok(())
    }

    /// Get a reasoning breadcrumb by ID.
    pub fn get_reasoning(&self, id: Uuid) -> Result<Option<ReasoningBreadcrumb>, PersistenceError> {
        let reasonings = self
            .reasonings
            .read()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        Ok(reasonings.get(&id).cloned())
    }

    /// List reasoning breadcrumbs for a turn.
    pub fn list_reasoning(
        &self,
        turn_id: Uuid,
    ) -> Result<Vec<ReasoningBreadcrumb>, PersistenceError> {
        let reasonings = self
            .reasonings
            .read()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        let reasonings_by_turn = self
            .reasonings_by_turn
            .read()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;

        if let Some(reasoning_ids) = reasonings_by_turn.get(&turn_id) {
            let mut result: Vec<ReasoningBreadcrumb> = reasoning_ids
                .iter()
                .filter_map(|id| reasonings.get(id).cloned())
                .collect();
            // Sort by sequence
            result.sort_by_key(|r| r.sequence);
            Ok(result)
        } else {
            Ok(Vec::new())
        }
    }

    // ==================== Batch Operations ====================

    /// Save a complete turn with all associated events.
    pub fn save_turn_with_events(
        &self,
        turn: TurnRecord,
        tool_events: Vec<ToolEventRecord>,
        reasonings: Vec<ReasoningBreadcrumb>,
    ) -> Result<(), PersistenceError> {
        // Save turn
        self.save_turn(turn)?;

        // Save tool events
        for event in tool_events {
            self.save_tool_event(event)?;
        }

        // Save reasonings
        for reasoning in reasonings {
            self.save_reasoning(reasoning)?;
        }

        Ok(())
    }

    /// Clear all data (for testing).
    pub fn clear(&self) -> Result<(), PersistenceError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        let mut turns = self
            .turns
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        let mut tool_events = self
            .tool_events
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        let mut reasonings = self
            .reasonings
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        let mut turns_by_session = self
            .turns_by_session
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        let mut tool_events_by_turn = self
            .tool_events_by_turn
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;
        let mut reasonings_by_turn = self
            .reasonings_by_turn
            .write()
            .map_err(|e| PersistenceError::Lock(e.to_string()))?;

        sessions.clear();
        turns.clear();
        tool_events.clear();
        reasonings.clear();
        turns_by_session.clear();
        tool_events_by_turn.clear();
        reasonings_by_turn.clear();

        Ok(())
    }
}

impl Default for PersistenceStore {
    fn default() -> Self {
        Self::new_in_memory()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_record_new() {
        let session = SessionRecord::new("user-123");
        assert_eq!(session.user_id(), "user-123");
        assert!(!session.id().is_nil());
    }

    #[test]
    fn test_turn_record_new() {
        let session_id = Uuid::new_v4();
        let turn = TurnRecord::new(session_id, 0, "Hello");
        assert_eq!(turn.session_id(), session_id);
        assert_eq!(turn.user_input(), "Hello");
        assert_eq!(turn.state(), TurnState::Processing);
    }

    #[test]
    fn test_turn_complete() {
        let session_id = Uuid::new_v4();
        let mut turn = TurnRecord::new(session_id, 0, "Hello");
        turn.complete("Hi there!");
        assert_eq!(turn.state(), TurnState::Completed);
        assert_eq!(turn.response(), Some("Hi there!"));
    }

    #[test]
    fn test_turn_fail() {
        let session_id = Uuid::new_v4();
        let mut turn = TurnRecord::new(session_id, 0, "Hello");
        turn.fail("Connection error");
        assert_eq!(turn.state(), TurnState::Failed);
        assert_eq!(turn.error(), Some("Connection error"));
    }

    #[test]
    fn test_tool_event_new() {
        let turn_id = Uuid::new_v4();
        let event = ToolEventRecord::new(turn_id, "search", serde_json::json!({"q": "test"}));
        assert_eq!(event.tool_name(), "search");
        assert!(event.result().is_none());
    }

    #[test]
    fn test_tool_event_with_result() {
        let turn_id = Uuid::new_v4();
        let mut event = ToolEventRecord::new(turn_id, "search", serde_json::json!({}));
        event.set_result(serde_json::json!({"results": []}));
        assert!(event.result().is_some());
    }

    #[test]
    fn test_should_externalize_large_payload() {
        let turn_id = Uuid::new_v4();
        let event = ToolEventRecord::new(turn_id, "read", serde_json::json!({}));

        let large_content = "x".repeat(100_000);
        let large_result = serde_json::json!({"content": large_content});

        assert!(event.should_externalize_result(&large_result));
    }

    #[test]
    fn test_should_not_externalize_small_payload() {
        let turn_id = Uuid::new_v4();
        let event = ToolEventRecord::new(turn_id, "echo", serde_json::json!({}));

        let small_result = serde_json::json!({"status": "ok"});

        assert!(!event.should_externalize_result(&small_result));
    }

    #[test]
    fn test_media_ref_new() {
        let media = MediaRef::new("media://test.png", "image/png", 1024);
        assert_eq!(media.uri(), "media://test.png");
        assert_eq!(media.content_type(), "image/png");
        assert_eq!(media.size_bytes(), 1024);
    }

    #[test]
    fn test_reasoning_breadcrumb_new() {
        let turn_id = Uuid::new_v4();
        let breadcrumb = ReasoningBreadcrumb::new(turn_id, "Thinking...");
        assert_eq!(breadcrumb.turn_id(), turn_id);
        assert_eq!(breadcrumb.content(), "Thinking...");
    }

    #[test]
    fn test_persistence_store_crud() {
        let store = PersistenceStore::new_in_memory();

        // Create and save session
        let session = SessionRecord::new("user-123");
        let session_id = session.id();
        store.save_session(session).unwrap();

        // Retrieve session
        let retrieved = store.get_session(session_id).unwrap().unwrap();
        assert_eq!(retrieved.user_id(), "user-123");

        // Delete session
        assert!(store.delete_session(session_id).unwrap());
        assert!(store.get_session(session_id).unwrap().is_none());
    }

    #[test]
    fn test_persistence_store_turns() {
        let store = PersistenceStore::new_in_memory();

        let session = SessionRecord::new("user-123");
        let session_id = session.id();
        store.save_session(session).unwrap();

        let turn = TurnRecord::new(session_id, 0, "Hello");
        let turn_id = turn.id();
        store.save_turn(turn).unwrap();

        let retrieved = store.get_turn(turn_id).unwrap().unwrap();
        assert_eq!(retrieved.user_input(), "Hello");

        let turns = store.list_turns(session_id).unwrap();
        assert_eq!(turns.len(), 1);
    }

    #[test]
    fn test_persistence_store_tool_events() {
        let store = PersistenceStore::new_in_memory();

        let session = SessionRecord::new("user-123");
        let session_id = session.id();
        store.save_session(session).unwrap();

        let turn = TurnRecord::new(session_id, 0, "Search");
        let turn_id = turn.id();
        store.save_turn(turn).unwrap();

        let event = ToolEventRecord::new(turn_id, "search", serde_json::json!({"q": "test"}));
        let event_id = event.id();
        store.save_tool_event(event).unwrap();

        let retrieved = store.get_tool_event(event_id).unwrap().unwrap();
        assert_eq!(retrieved.tool_name(), "search");

        let events = store.list_tool_events(turn_id).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_persistence_store_reasonings() {
        let store = PersistenceStore::new_in_memory();

        let session = SessionRecord::new("user-123");
        let session_id = session.id();
        store.save_session(session).unwrap();

        let turn = TurnRecord::new(session_id, 0, "Analyze");
        let turn_id = turn.id();
        store.save_turn(turn).unwrap();

        let reasoning = ReasoningBreadcrumb::new(turn_id, "Step 1...");
        let reasoning_id = reasoning.id();
        store.save_reasoning(reasoning).unwrap();

        let retrieved = store.get_reasoning(reasoning_id).unwrap().unwrap();
        assert_eq!(retrieved.content(), "Step 1...");

        let reasonings = store.list_reasoning(turn_id).unwrap();
        assert_eq!(reasonings.len(), 1);
    }
}
