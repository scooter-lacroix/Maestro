//! Normalized model types for Conductor tree rendering
//!
//! Provides tree-based data structures for rendering tracks, tasks, and sessions.

//! Normalized model types for Conductor tree rendering
//!
//! Provides tree-based data structures for rendering tracks, tasks, and sessions.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use leindex_core::orchestrate::model::{SessionState, TrackMetadata, TrackPlan};

/// Unique identifier for a node in the conductor tree
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TreeNodeId {
    Root,
    Track(String),
    Task { track_id: String, task_id: String },
    Session { track_id: String, task_id: String, session_id: String },
}

impl TreeNodeId {
    pub fn track(id: impl Into<String>) -> Self {
        Self::Track(id.into())
    }

    pub fn task(id: impl Into<String>) -> Self {
        Self::Task { track_id: String::new(), task_id: id.into() }
    }

    pub fn session(id: impl Into<String>) -> Self {
        Self::Session {
            track_id: String::new(),
            task_id: String::new(),
            session_id: id.into(),
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            TreeNodeId::Root => "root".to_string(),
            TreeNodeId::Track(id) => format!("track:{}", id),
            TreeNodeId::Task { track_id, task_id } => {
                if track_id.is_empty() {
                    format!("task:{}", task_id)
                } else {
                    format!("track:{}/task:{}", track_id, task_id)
                }
            }
            TreeNodeId::Session { track_id, task_id, session_id } => {
                format!("track:{}/task:{}/session:{}", track_id, task_id, session_id)
            }
        }
    }
}

/// Status of a conductor node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConductorNodeStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Running,
    Paused,
    Failed,
    Idle,
    Unknown,
}

impl From<leindex_core::orchestrate::model::SessionStatus> for ConductorNodeStatus {
    fn from(status: leindex_core::orchestrate::model::SessionStatus) -> Self {
        match status {
            leindex_core::orchestrate::model::SessionStatus::Idle => ConductorNodeStatus::Idle,
            leindex_core::orchestrate::model::SessionStatus::Running => ConductorNodeStatus::Running,
            leindex_core::orchestrate::model::SessionStatus::Pausing => ConductorNodeStatus::Paused,
            leindex_core::orchestrate::model::SessionStatus::Paused => ConductorNodeStatus::Paused,
            leindex_core::orchestrate::model::SessionStatus::Completed => ConductorNodeStatus::Completed,
            leindex_core::orchestrate::model::SessionStatus::Failed => ConductorNodeStatus::Failed,
            leindex_core::orchestrate::model::SessionStatus::Interrupted => ConductorNodeStatus::Failed,
            leindex_core::orchestrate::model::SessionStatus::Stopping => ConductorNodeStatus::Running,
        }
    }
}

/// External session representation
#[derive(Debug, Clone)]
pub struct ExternalSession {
    pub id: String,
    pub track_id: String,
    pub task_id: String,
    pub title: String,
    pub status: ConductorNodeStatus,
}

/// Trait for conductor tree nodes
pub trait ConductorNode: Send + Sync + std::fmt::Debug {
    fn status(&self) -> ConductorNodeStatus;
    fn is_expandable(&self) -> bool;
    fn title(&self) -> &str;
    fn children(&self) -> Vec<Arc<dyn ConductorNode>>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn id(&self) -> &str;
}

/// A track node in the conductor tree
#[derive(Debug, Clone)]
pub struct ConductorTrackNode {
    pub id: String,
    pub title: String,
    pub status: ConductorNodeStatus,
    pub children: Vec<Arc<dyn ConductorNode>>,
}

/// A task node in the conductor tree
#[derive(Debug, Clone)]
pub struct ConductorTaskNode {
    pub id: String,
    pub track_id: String,
    pub title: String,
    pub description: String,
    pub status: ConductorNodeStatus,
    pub children: Vec<Arc<dyn ConductorNode>>,
}

impl ConductorNode for ConductorTaskNode {
    fn status(&self) -> ConductorNodeStatus {
        self.status
    }

    fn is_expandable(&self) -> bool {
        !self.children.is_empty()
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn children(&self) -> Vec<Arc<dyn ConductorNode>> {
        self.children.clone()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }
}

/// A session node in the conductor tree
#[derive(Debug, Clone)]
pub struct ConductorSessionNode {
    pub id: String,
    pub track_id: String,
    pub task_id: String,
    pub title: String,
    pub status: ConductorNodeStatus,
    pub iteration: u64,
}

impl ConductorNode for ConductorSessionNode {
    fn status(&self) -> ConductorNodeStatus {
        self.status
    }

    fn is_expandable(&self) -> bool {
        false
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn children(&self) -> Vec<Arc<dyn ConductorNode>> {
        Vec::new()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }
}

impl ConductorNode for ConductorTrackNode {
    fn status(&self) -> ConductorNodeStatus {
        self.status
    }

    fn is_expandable(&self) -> bool {
        !self.children.is_empty()
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn children(&self) -> Vec<Arc<dyn ConductorNode>> {
        self.children.clone()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }
}

/// Configuration for tree builder
#[derive(Debug, Clone, Default)]
pub struct TreeBuilderConfig {
    pub max_depth: usize,
    pub show_status: bool,
}

/// Tree structure for conductor rendering
#[derive(Debug, Clone, Default)]
pub struct ConductorTree {
    pub roots: Vec<Arc<dyn ConductorNode>>,
    pub nodes: HashMap<TreeNodeId, Arc<dyn ConductorNode>>,
    pub root_ids: Vec<TreeNodeId>,
    pub expanded_nodes: HashSet<TreeNodeId>,
    pub selected_node: Option<TreeNodeId>,
}

impl ConductorTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle_expanded(&mut self, node_id: &TreeNodeId) {
        if self.expanded_nodes.contains(node_id) {
            self.expanded_nodes.remove(node_id);
        } else {
            self.expanded_nodes.insert(node_id.clone());
        }
    }

    pub fn is_expanded(&self, node_id: &TreeNodeId) -> bool {
        self.expanded_nodes.contains(node_id)
    }

    pub fn selected(&self) -> Option<&TreeNodeId> {
        self.selected_node.as_ref()
    }

    pub fn set_selected(&mut self, node_id: Option<TreeNodeId>) {
        self.selected_node = node_id;
    }

    pub fn visible_nodes(&self) -> Vec<(TreeNodeId, Arc<dyn ConductorNode>)> {
        let mut result = Vec::new();
        for root_id in &self.root_ids {
            if let Some(node) = self.nodes.get(root_id) {
                self.collect_visible_nodes(root_id.clone(), node.clone(), &mut result);
            }
        }
        result
    }

    fn collect_visible_nodes(
        &self,
        id: TreeNodeId,
        node: Arc<dyn ConductorNode>,
        result: &mut Vec<(TreeNodeId, Arc<dyn ConductorNode>)>,
    ) {
        result.push((id.clone(), node.clone()));
        if self.expanded_nodes.contains(&id) {
            for child in node.children() {
                let child_id = TreeNodeId::Track(child.id().to_string());
                self.collect_visible_nodes(child_id, child, result);
            }
        }
    }

    pub fn add_node(&mut self, id: TreeNodeId, node: Arc<dyn ConductorNode>, is_root: bool) {
        if is_root {
            self.root_ids.push(id.clone());
            self.roots.push(node.clone());
        }
        self.nodes.insert(id, node);
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.root_ids.clear();
        self.roots.clear();
        self.expanded_nodes.clear();
        self.selected_node = None;
    }
}

/// Tree builder for constructing conductor trees
pub struct TreeBuilder {
    tree: ConductorTree,
    config: TreeBuilderConfig,
    tracks: Vec<leindex_core::orchestrate::model::Track>,
    metadata: HashMap<String, TrackMetadata>,
    plans: HashMap<String, TrackPlan>,
    sessions: Vec<SessionState>,
    external_sessions: Vec<ExternalSession>,
}

impl TreeBuilder {
    pub fn new() -> Self {
        Self {
            tree: ConductorTree::new(),
            config: TreeBuilderConfig::default(),
            tracks: Vec::new(),
            metadata: HashMap::new(),
            plans: HashMap::new(),
            sessions: Vec::new(),
            external_sessions: Vec::new(),
        }
    }

    pub fn with_config(mut self, config: TreeBuilderConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_tracks(mut self, tracks: Vec<leindex_core::orchestrate::model::Track>) -> Self {
        self.tracks = tracks;
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, TrackMetadata>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_plans(mut self, plans: HashMap<String, TrackPlan>) -> Self {
        self.plans = plans;
        self
    }

    pub fn with_sessions(mut self, sessions: Vec<SessionState>) -> Self {
        self.sessions = sessions;
        self
    }

    pub fn with_external_sessions(mut self, sessions: Vec<ExternalSession>) -> Self {
        self.external_sessions = sessions;
        self
    }

    pub fn build(mut self) -> Result<ConductorTree, String> {
        // Build task nodes first (collect to avoid borrow issues)
        let mut all_task_nodes: Vec<(TreeNodeId, Arc<dyn ConductorNode>)> = Vec::new();

        for track in &self.tracks {
            if let Some(plan) = self.plans.get(&track.id) {
                for task in &plan.tasks {
                    Self::collect_task_nodes(
                        &track.id,
                        task,
                        &mut all_task_nodes,
                    );
                }
            }
        }

        // Add all task nodes to tree
        for (id, node) in &all_task_nodes {
            self.tree.add_node(id.clone(), node.clone(), false);
        }

        // Collect all session nodes first (before building tracks)
        let mut all_session_nodes: Vec<(TreeNodeId, Arc<dyn ConductorNode>)> = Vec::new();
        let mut sessions_by_track: HashMap<String, Vec<Arc<dyn ConductorNode>>> = HashMap::new();

        // Collect internal sessions
        for session in &self.sessions {
            let session_node: Arc<dyn ConductorNode> = Arc::new(ConductorSessionNode {
                id: session.session_id.clone(),
                track_id: session.track_id.clone(),
                task_id: session.current_task_id.clone().unwrap_or_default(),
                title: format!("Session {} ({:?})", session.session_id, session.status),
                status: session.status.into(),
                iteration: session.current_iteration,
            });
            let session_id = TreeNodeId::Session {
                track_id: session.track_id.clone(),
                task_id: session.current_task_id.clone().unwrap_or_default(),
                session_id: session.session_id.clone(),
            };
            all_session_nodes.push((session_id, session_node.clone()));
            sessions_by_track
                .entry(session.track_id.clone())
                .or_default()
                .push(session_node);
        }

        // Collect external sessions
        for session in &self.external_sessions {
            let session_node: Arc<dyn ConductorNode> = Arc::new(ConductorSessionNode {
                id: session.id.clone(),
                track_id: session.track_id.clone(),
                task_id: session.task_id.clone(),
                title: format!("External: {}", session.title),
                status: session.status,
                iteration: 0,
            });
            let session_id = TreeNodeId::Session {
                track_id: session.track_id.clone(),
                task_id: session.task_id.clone(),
                session_id: session.id.clone(),
            };
            all_session_nodes.push((session_id, session_node.clone()));
            sessions_by_track
                .entry(session.track_id.clone())
                .or_default()
                .push(session_node);
        }

        // Add all session nodes to tree
        for (id, node) in all_session_nodes {
            self.tree.add_node(id, node, false);
        }

        // Build track nodes with their children
        for track in &self.tracks {
            let track_id = TreeNodeId::Track(track.id.clone());

            // Determine status from track's status field
            let status = match track.status {
                leindex_core::orchestrate::model::TrackStatus::Pending => ConductorNodeStatus::Pending,
                leindex_core::orchestrate::model::TrackStatus::InProgress => ConductorNodeStatus::InProgress,
                leindex_core::orchestrate::model::TrackStatus::Completed => ConductorNodeStatus::Completed,
            };

            // Get metadata for additional track info
            let metadata = self.metadata.get(&track.id);
            let track_type = metadata
                .map(|m| match m.track_type {
                    leindex_core::orchestrate::model::TrackType::Feature => "Feature",
                    leindex_core::orchestrate::model::TrackType::Master => "Master",
                    leindex_core::orchestrate::model::TrackType::Refactor => "Refactor",
                    leindex_core::orchestrate::model::TrackType::Hotfix => "Hotfix",
                })
                .unwrap_or("");

            // Collect task children for this track
            let mut task_children: Vec<Arc<dyn ConductorNode>> = Vec::new();

            // Add top-level tasks from plan
            if let Some(plan) = self.plans.get(&track.id) {
                for task in &plan.tasks {
                    let task_id = TreeNodeId::Task {
                        track_id: track.id.clone(),
                        task_id: task.id.clone(),
                    };
                    if let Some((_, node)) = all_task_nodes.iter().find(|(id, _)| *id == task_id) {
                        task_children.push(node.clone());
                    }
                }
            }

            // Add session nodes for this track
            if let Some(sessions) = sessions_by_track.get(&track.id) {
                task_children.extend(sessions.iter().cloned());
            }

            let title = if track.description.is_empty() {
                if track_type.is_empty() {
                    track.id.clone()
                } else {
                    format!("[{}] {}", track_type, track.id)
                }
            } else {
                if track_type.is_empty() {
                    track.description.clone()
                } else {
                    format!("[{}] {}", track_type, track.description)
                }
            };

            let node = Arc::new(ConductorTrackNode {
                id: track.id.clone(),
                title,
                status,
                children: task_children,
            });
            self.tree.add_node(track_id, node, true);
        }

        Ok(self.tree.clone())
    }

    fn collect_task_nodes(
        track_id: &str,
        task: &leindex_core::orchestrate::model::Task,
        result: &mut Vec<(TreeNodeId, Arc<dyn ConductorNode>)>,
    ) {
        // Determine task status
        let status = match task.status {
            leindex_core::orchestrate::model::TrackStatus::Pending => ConductorNodeStatus::Pending,
            leindex_core::orchestrate::model::TrackStatus::InProgress => ConductorNodeStatus::InProgress,
            leindex_core::orchestrate::model::TrackStatus::Completed => ConductorNodeStatus::Completed,
        };

        // Build subtask children first
        let mut subtask_children: Vec<Arc<dyn ConductorNode>> = Vec::new();
        for subtask in &task.subtasks {
            let subtask_id = TreeNodeId::Task {
                track_id: track_id.to_string(),
                task_id: subtask.id.clone(),
            };
            // Recursively collect subtask nodes
            Self::collect_task_nodes(track_id, subtask, result);
            // Find the subtask node to add as child
            if let Some((_, node)) = result.iter().find(|(id, _)| *id == subtask_id) {
                subtask_children.push(node.clone());
            }
        }

        let task_id = TreeNodeId::Task {
            track_id: track_id.to_string(),
            task_id: task.id.clone(),
        };

        let node = Arc::new(ConductorTaskNode {
            id: task.id.clone(),
            track_id: track_id.to_string(),
            title: task.title.clone(),
            description: task.description.clone(),
            status,
            children: subtask_children,
        });
        result.push((task_id, node));
    }
}

