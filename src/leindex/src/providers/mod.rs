//! Concrete provider adapters for the provider-boundary migration.

pub mod nexus_bridge;
pub mod standalone_leindex;
pub mod standalone_nexus;

pub use nexus_bridge::NexusRuntimeBridge;
pub use standalone_leindex::{LeIndexInstallMethod, StandaloneLeIndexProvider};
pub use standalone_nexus::{NexusInstallMethod, StandaloneNexusProvider};
