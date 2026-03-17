//! Observer module for conductor event handling
//!
//! Provides event bridge and observer actions for session management.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use super::model::ConductorEvent;

/// Maximum number of events to buffer per session
const EVENT_BUFFER_SIZE: usize = 256;

/// Trait for session event bridging
pub trait SessionEventBridge: Send + Sync {
    /// Subscribe to events for a session
    fn subscribe(&self, session_id: &str) -> broadcast::Receiver<ConductorEvent>;
    
    /// Publish an event to a session
    fn publish(&self, session_id: &str, event: ConductorEvent) -> Result<(), String>;
}

/// In-memory implementation of SessionEventBridge
pub struct InMemoryEventBridge {
    senders: RwLock<HashMap<String, broadcast::Sender<ConductorEvent>>>,
}

impl InMemoryEventBridge {
    /// Create a new InMemoryEventBridge
    pub fn new() -> Self {
        Self {
            senders: RwLock::new(HashMap::new()),
        }
    }
    
    fn get_or_create_sender(&self, session_id: &str) -> broadcast::Sender<ConductorEvent> {
        let senders = self.senders.read().expect("RwLock read lock poisoned");
        if let Some(sender) = senders.get(session_id) {
            return sender.clone();
        }
        drop(senders);

        let mut senders = self.senders.write().expect("RwLock write lock poisoned");
        // Re-check after acquiring write lock to prevent race condition.
        if let Some(sender) = senders.get(session_id) {
            return sender.clone();
        }

        let sender = broadcast::channel(EVENT_BUFFER_SIZE).0;
        senders.insert(session_id.to_string(), sender.clone());
        sender
    }
}

impl Default for InMemoryEventBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionEventBridge for InMemoryEventBridge {
    fn subscribe(&self, session_id: &str) -> broadcast::Receiver<ConductorEvent> {
        self.get_or_create_sender(session_id).subscribe()
    }
    
    fn publish(&self, session_id: &str, event: ConductorEvent) -> Result<(), String> {
        let sender = self.get_or_create_sender(session_id);
        sender.send(event).map(|_| ()).map_err(|e| e.to_string())
    }
}

/// Actions that an observer can take during orchestration
#[derive(Debug, Clone, PartialEq)]
pub enum ObserverAction {
    /// Review the current task
    ReviewCurrentTask {
        iteration: u64,
        task_id: String,
    },
    /// Request a retry of a task
    RequestRetry {
        task_id: String,
        reason: String,
    },
    /// Inject guidance into a task
    InjectGuidance {
        task_id: String,
        guidance: String,
    },
    /// Pause the orchestration
    RequestPause {
        reason: String,
    },
    /// Approve and continue
    ApproveAndContinue {
        task_id: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_event_bridge_creation() {
        let bridge = InMemoryEventBridge::new();
        assert!(bridge.senders.read().unwrap().is_empty());
    }

    #[test]
    fn test_observer_action_variants() {
        let action1 = ObserverAction::ReviewCurrentTask {
            iteration: 1,
            task_id: "task-1".to_string(),
        };
        
        let action2 = ObserverAction::RequestRetry {
            task_id: "task-1".to_string(),
            reason: "Temporary error".to_string(),
        };
        
        let action3 = ObserverAction::InjectGuidance {
            task_id: "task-1".to_string(),
            guidance: "Consider using async/await".to_string(),
        };
        
        // Verify variants are distinct
        assert_ne!(action1, action2);
        assert_ne!(action2, action3);
    }
}
