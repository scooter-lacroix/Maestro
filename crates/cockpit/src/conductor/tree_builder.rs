//! Tree builder module for constructing conductor trees
//!
//! Re-exports from normalized_model for backward compatibility.

// Re-export all types from normalized_model
pub use super::normalized_model::{
    ConductorNode, ConductorNodeStatus, ConductorSessionNode, ConductorTaskNode,
    ConductorTrackNode, ConductorTree, ExternalSession, TreeBuilder, TreeBuilderConfig, TreeNodeId,
};
