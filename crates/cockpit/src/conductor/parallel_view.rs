//! Parallel execution view component

use leindex_core::orchestrate::model::ParallelGroupInfo;
use leindex_core::orchestrate::model::WorkerStatus;

/// Parallel view component displaying worker status and merge queue
#[derive(Debug, Clone, Default)]
pub struct ParallelView {
    /// The parallel group being displayed
    pub group_info: Option<ParallelGroupInfo>,
    /// Currently selected worker ID
    pub selected_worker: Option<String>,
    /// Scroll offset for the view
    pub scroll_offset: u16,
}

impl ParallelView {
    /// Create a new parallel view
    pub fn new() -> Self {
        Self::default()
    }

    /// Update with group info
    pub fn update(&mut self, group_info: ParallelGroupInfo) {
        self.group_info = Some(group_info);
    }

    /// Select a worker by ID
    pub fn select_worker(&mut self, worker_id: String) {
        self.selected_worker = Some(worker_id);
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected_worker = None;
    }

    /// Get status icon for a worker
    pub fn get_status_icon(status: &WorkerStatus) -> &'static str {
        match status {
            WorkerStatus::Idle => "[ ]",
            WorkerStatus::Working => "[~]",
            WorkerStatus::Waiting => "[?]",
            WorkerStatus::Complete => "[x]",
            WorkerStatus::Error => "[!]",
        }
    }
}
