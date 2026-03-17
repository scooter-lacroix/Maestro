//! Tree builder module for constructing conductor trees
//!
//! Re-exports from normalized_model for backward compatibility.

pub use super::normalized_model::{
    TreeBuilder, TreeBuilderConfig, ConductorTree, ConductorNode,
    ConductorNodeStatus, TreeNodeId, ConductorTrackNode, ExternalSession,
};
