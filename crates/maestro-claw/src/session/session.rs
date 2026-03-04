//! Session model - Top-level conversation container
//!
//! A Session represents a complete conversation with a user,
//! containing multiple threads and metadata.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::Thread;

/// Metadata associated with a session
pub type SessionMetadata = HashMap<String, String>;

/// Top-level container for conversation state
///
/// A Session contains:
/// - One or more threads (conversation branches)
/// - Metadata (user info, model config, etc.)
/// - Creation timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier for this session
    pub id: String,
    /// Threads within this session
    pub threads: Vec<Thread>,
    /// Arbitrary metadata
    #[serde(default)]
    pub metadata: SessionMetadata,
    /// When this session was created
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
}

impl Session {
    /// Create a new session with a generated ID
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            threads: Vec::new(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Create a new session with a specific ID
    pub fn with_id(id: String) -> Self {
        Self {
            id,
            threads: Vec::new(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Create a new session with a display name
    pub fn named(title: impl Into<String>) -> Self {
        let mut session = Self::new();
        session.metadata.insert("title".to_string(), title.into());
        session
    }

    /// Get the session title (if set)
    pub fn title(&self) -> Option<&str> {
        self.metadata.get("title").map(|s| s.as_str())
    }

    /// Get the session ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get an iterator over threads
    pub fn threads(&self) -> impl Iterator<Item = &Thread> {
        self.threads.iter()
    }

    /// Get a thread by ID
    pub fn get_thread(&self, id: &str) -> Option<&Thread> {
        self.threads.iter().find(|t| t.id() == id)
    }

    /// Get a mutable reference to a thread by ID
    pub fn get_thread_mut(&mut self, id: &str) -> Option<&mut Thread> {
        self.threads.iter_mut().find(|t| t.id() == id)
    }

    /// Add a new thread and return a mutable reference to it
    pub fn add_thread(&mut self) -> &mut Thread {
        let thread = Thread::new(self.id.clone());
        self.threads.push(thread);
        self.threads.last_mut().unwrap()
    }

    /// Get the metadata
    pub fn metadata(&self) -> &SessionMetadata {
        &self.metadata
    }

    /// Get mutable access to metadata
    pub fn metadata_mut(&mut self) -> &mut SessionMetadata {
        &mut self.metadata
    }

    /// Get the creation timestamp
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Get the number of threads
    pub fn thread_count(&self) -> usize {
        self.threads.len()
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new();
        assert!(!session.id().is_empty());
        assert_eq!(session.thread_count(), 0);
    }

    #[test]
    fn test_session_with_id() {
        let session = Session::with_id("custom-id".to_string());
        assert_eq!(session.id(), "custom-id");
    }

    #[test]
    fn test_session_add_thread() {
        let mut session = Session::new();
        let session_id = session.id().to_string();
        session.add_thread();
        assert_eq!(session.thread_count(), 1);
        let thread = session.threads.first().unwrap();
        assert_eq!(thread.session_id(), session_id);
    }

    #[test]
    fn test_session_get_thread() {
        let mut session = Session::new();
        session.add_thread();
        let thread_id = session.threads.first().unwrap().id().to_string();

        let found = session.get_thread(&thread_id);
        assert!(found.is_some());

        let missing = session.get_thread("non-existent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_session_add_thread_returns_mutable() {
        use super::super::Turn;
        use super::super::TurnRole;

        let mut session = Session::new();
        // add_thread() returns &mut Thread — can add turns directly
        let thread = session.add_thread();
        thread.add_turn(Turn::new(TurnRole::User, "hello".to_string()));
        assert_eq!(session.threads.first().unwrap().turn_count(), 1);
    }

    #[test]
    fn test_session_metadata() {
        let mut session = Session::new();
        session.metadata_mut().insert("user".to_string(), "alice".to_string());

        assert_eq!(session.metadata().get("user"), Some(&"alice".to_string()));
    }

    #[test]
    fn test_session_serialization() {
        let session = Session::new();
        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("threads"));
    }

    #[test]
    fn test_session_deserialization() {
        let json = r#"{
            "id": "test-id",
            "threads": [],
            "metadata": {"key": "value"},
            "created_at": "2026-02-23T12:00:00Z"
        }"#;
        let session: Session = serde_json::from_str(json).unwrap();
        assert_eq!(session.id(), "test-id");
        assert_eq!(session.metadata().get("key"), Some(&"value".to_string()));
    }
}
