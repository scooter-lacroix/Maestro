//! Session management for Maestro-tab integration

use anyhow::Result;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tab_api::tab::{CreateTabMetadata, TabId, TabMetadata};

/// A Maestro terminal session backed by tab-rs
#[derive(Debug, Clone)]
pub struct MaestroSession {
    /// Unique session identifier
    pub id: TabId,
    /// Session name (human-readable)
    pub name: String,
    /// Working directory
    pub work_dir: String,
    /// Shell command
    pub shell: String,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Terminal dimensions (cols, rows)
    pub dimensions: (u16, u16),
    /// Creation timestamp
    pub created_at: std::time::Instant,
}

impl MaestroSession {
    /// Create a new session from tab-rs metadata
    pub fn from_metadata(metadata: &TabMetadata) -> Self {
        Self {
            id: metadata.id,
            name: metadata.name.clone(),
            work_dir: metadata.doc.clone().unwrap_or_default(),
            shell: String::new(),
            env: metadata.env.clone(),
            dimensions: metadata.dimensions,
            created_at: std::time::Instant::now(),
        }
    }

    /// Create metadata for a new tab creation request
    pub fn to_create_metadata(&self) -> CreateTabMetadata {
        CreateTabMetadata {
            name: self.name.clone(),
            dimensions: self.dimensions,
            doc: Some(self.work_dir.clone()),
            env: self.env.clone(),
            shell: self.shell.clone(),
            dir: self.work_dir.clone(),
        }
    }
}

/// Session manager that tracks all active Maestro sessions
pub struct SessionManager {
    /// Active sessions indexed by name
    sessions: Arc<DashMap<String, MaestroSession>>,
    /// Session name to TabId mapping
    name_to_id: Arc<DashMap<String, TabId>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            name_to_id: Arc::new(DashMap::new()),
        }
    }

    /// Register a new session
    pub fn register(&self, session: MaestroSession) {
        let name = session.name.clone();
        let id = session.id;
        self.sessions.insert(name.clone(), session);
        self.name_to_id.insert(name, id);
    }

    /// Unregister a session by name
    pub fn unregister(&self, name: &str) -> Option<MaestroSession> {
        self.name_to_id.remove(name);
        self.sessions.remove(name).map(|(_, s)| s)
    }

    /// Get a session by name
    pub fn get(&self, name: &str) -> Option<MaestroSession> {
        self.sessions.get(name).map(|r| r.value().clone())
    }

    /// Get a session by ID
    pub fn get_by_id(&self, id: TabId) -> Option<MaestroSession> {
        self.sessions
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.value().clone())
    }

    /// List all session names
    pub fn list_sessions(&self) -> Vec<String> {
        self.sessions.iter().map(|r| r.key().clone()).collect()
    }

    /// Check if a session exists
    pub fn exists(&self, name: &str) -> bool {
        self.sessions.contains_key(name)
    }

    /// Get the number of active sessions
    pub fn count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_manager_register() {
        let manager = SessionManager::new();
        let session = MaestroSession {
            id: TabId(1),
            name: "test".to_string(),
            work_dir: "/tmp".to_string(),
            shell: "/bin/bash".to_string(),
            env: HashMap::new(),
            dimensions: (80, 24),
            created_at: std::time::Instant::now(),
        };

        manager.register(session);
        assert!(manager.exists("test"));
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_session_manager_unregister() {
        let manager = SessionManager::new();
        let session = MaestroSession {
            id: TabId(1),
            name: "test".to_string(),
            work_dir: "/tmp".to_string(),
            shell: "/bin/bash".to_string(),
            env: HashMap::new(),
            dimensions: (80, 24),
            created_at: std::time::Instant::now(),
        };

        manager.register(session);
        let removed = manager.unregister("test");
        assert!(removed.is_some());
        assert!(!manager.exists("test"));
    }
}
