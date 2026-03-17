//! Normalized model types for Conductor tree rendering
//!
//! Provides tree-based data structures for rendering tracks, tasks, and sessions.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
}

impl TreeBuilder {
    pub fn new() -> Self {
        Self {
            tree: ConductorTree::new(),
            config: TreeBuilderConfig::default(),
        }
    }

    pub fn with_config(config: TreeBuilderConfig) -> Self {
        Self {
            tree: ConductorTree::new(),
            config,
        }
    }

    pub fn add_track(&mut self, id: String, title: String, status: ConductorNodeStatus) {
        let node = Arc::new(ConductorTrackNode {
            id: id.clone(),
            title,
            status,
            children: Vec::new(),
        });
        self.tree.add_node(TreeNodeId::Track(id), node, true);
    }

    pub fn build(self) -> ConductorTree {
        self.tree
    }
}

impl Default for TreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
