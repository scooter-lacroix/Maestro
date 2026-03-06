//! Track Integration Module
//!
//! Wires together the approval flow, memory search, and track task completion
//! for spec-driven development workflow.

use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// Track task status for approval workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is pending execution
    Pending,
    /// Task is in progress
    InProgress,
    /// Task is awaiting approval
    AwaitingApproval,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task was skipped
    Skipped,
}

/// Track context for approval requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackContext {
    /// Track ID (e.g., "overhaul_20260217")
    pub track_id: String,
    /// Task ID within the track
    pub task_id: String,
    /// Task description
    pub task_description: Option<String>,
    /// Current task status
    pub status: TaskStatus,
    /// Approval request ID if awaiting approval
    pub approval_request_id: Option<String>,
    /// Timestamp of last status change
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl TrackContext {
    /// Create a new track context
    pub fn new(track_id: impl Into<String>, task_id: impl Into<String>) -> Self {
        Self {
            track_id: track_id.into(),
            task_id: task_id.into(),
            task_description: None,
            status: TaskStatus::Pending,
            approval_request_id: None,
            updated_at: chrono::Utc::now(),
        }
    }

    /// Set the task description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.task_description = Some(description.into());
        self
    }

    /// Update the task status
    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = status;
        self.updated_at = chrono::Utc::now();
    }

    /// Mark as awaiting approval
    pub fn set_awaiting_approval(&mut self, request_id: impl Into<String>) {
        self.status = TaskStatus::AwaitingApproval;
        self.approval_request_id = Some(request_id.into());
        self.updated_at = chrono::Utc::now();
    }

    /// Clear approval request
    pub fn clear_approval(&mut self) {
        self.approval_request_id = None;
    }
}

/// Memory search context with track awareness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchContext {
    /// Track context if searching within a track
    pub track: Option<TrackContext>,
    /// Search query
    pub query: String,
    /// Maximum results
    pub limit: usize,
    /// Whether to include track-specific memories
    pub include_track_memories: bool,
}

impl MemorySearchContext {
    /// Create a new memory search context
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            track: None,
            query: query.into(),
            limit: 10,
            include_track_memories: true,
        }
    }

    /// Set the track context
    pub fn with_track(mut self, track: TrackContext) -> Self {
        self.track = Some(track);
        self
    }

    /// Set the result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Build the augmented query with track context
    pub fn augmented_query(&self) -> String {
        if let Some(ref track) = self.track {
            format!(
                "track:{} task:{} {}",
                track.track_id, track.task_id, self.query
            )
        } else {
            self.query.clone()
        }
    }
}

/// Approval-workflow integration manager
pub struct ApprovalTrackIntegration {
    /// Active track contexts by request ID
    pending_approvals: RwLock<std::collections::HashMap<String, TrackContext>>,
}

impl ApprovalTrackIntegration {
    /// Create a new integration manager
    pub fn new() -> Self {
        Self {
            pending_approvals: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Register an approval request with track context
    pub fn register_approval(&self, request_id: &str, context: TrackContext) {
        let mut pending = self.pending_approvals.write().unwrap();
        pending.insert(request_id.to_string(), context);
    }

    /// Handle an approval decision
    pub fn handle_decision(&self, request_id: &str, approved: bool) -> Option<TrackContext> {
        let mut pending = self.pending_approvals.write().unwrap();

        if let Some(mut context) = pending.remove(request_id) {
            if approved {
                context.set_status(TaskStatus::InProgress);
            } else {
                context.set_status(TaskStatus::Failed);
            }
            context.clear_approval();
            Some(context)
        } else {
            None
        }
    }

    /// Get pending approval context
    pub fn get_pending(&self, request_id: &str) -> Option<TrackContext> {
        let pending = self.pending_approvals.read().unwrap();
        pending.get(request_id).cloned()
    }

    /// List all pending approvals
    pub fn list_pending(&self) -> Vec<(String, TrackContext)> {
        let pending = self.pending_approvals.read().unwrap();
        pending
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Clear stale pending approvals (older than specified duration)
    pub fn clear_stale(&self, max_age: chrono::Duration) -> usize {
        let mut pending = self.pending_approvals.write().unwrap();
        let now = chrono::Utc::now();
        let initial_len = pending.len();

        pending.retain(|_, ctx| now.signed_duration_since(ctx.updated_at) < max_age);

        initial_len - pending.len()
    }
}

impl Default for ApprovalTrackIntegration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_context_creation() {
        let ctx = TrackContext::new("test_track", "task_1");
        assert_eq!(ctx.track_id, "test_track");
        assert_eq!(ctx.task_id, "task_1");
        assert_eq!(ctx.status, TaskStatus::Pending);
    }

    #[test]
    fn test_track_context_status_update() {
        let mut ctx = TrackContext::new("test_track", "task_1");
        ctx.set_status(TaskStatus::InProgress);
        assert_eq!(ctx.status, TaskStatus::InProgress);
    }

    #[test]
    fn test_memory_search_context_augmented_query() {
        let track = TrackContext::new("overhaul_20260217", "task_4_1");
        let search = MemorySearchContext::new("approval flow").with_track(track);

        let augmented = search.augmented_query();
        assert!(augmented.contains("track:overhaul_20260217"));
        assert!(augmented.contains("task:task_4_1"));
        assert!(augmented.contains("approval flow"));
    }

    #[test]
    fn test_approval_track_integration() {
        let integration = ApprovalTrackIntegration::new();

        let ctx = TrackContext::new("test_track", "task_1").with_description("Test task");
        integration.register_approval("req_123", ctx);

        let pending = integration.get_pending("req_123");
        assert!(pending.is_some());

        let result = integration.handle_decision("req_123", true);
        assert!(result.is_some());
        assert_eq!(result.unwrap().status, TaskStatus::InProgress);

        let pending = integration.get_pending("req_123");
        assert!(pending.is_none());
    }

    #[test]
    fn test_integration_list_pending() {
        let integration = ApprovalTrackIntegration::new();

        integration.register_approval("req_1", TrackContext::new("track_1", "task_1"));
        integration.register_approval("req_2", TrackContext::new("track_1", "task_2"));

        let pending = integration.list_pending();
        assert_eq!(pending.len(), 2);
    }
}
