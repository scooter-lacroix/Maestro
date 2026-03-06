//! Integration layer for maestro-claw with maestro-core
//!
//! This module provides bridges between maestro-claw and maestro-core:
//! - `security` - SecurityPolicy enforcement for tool execution
//! - `memory` - Memory trait integration for persistent storage
//! - `channel` - Channel trait integration for message routing
//!
//! These integrations are optional and require the `core-integration` feature.

#[cfg(feature = "core-integration")]
pub mod channel;
#[cfg(feature = "core-integration")]
pub mod memory;
#[cfg(feature = "core-integration")]
pub mod security;

// Re-export integration types when feature is enabled
#[cfg(feature = "core-integration")]
pub use channel::{ChannelBridge, ChannelBridgeError};
#[cfg(feature = "core-integration")]
pub use memory::{MemoryBridge, MemoryBridgeError};
#[cfg(feature = "core-integration")]
pub use security::{ApprovalCallback, SecurityPolicyBridge, SecurityPolicyError};

#[cfg(feature = "core-integration")]
pub use maestro_core::capabilities::sandbox::{
    AutonomyLevel, ExecutionRequest, ResourceLimits, RuntimeAdapter, SandboxManager, SandboxResult,
    SecurityPolicy,
};
#[cfg(feature = "core-integration")]
pub use maestro_core::channel::{
    Channel, ChannelPlugin, ChannelRegistry, IncomingMessage, OutgoingResponse,
};
#[cfg(feature = "core-integration")]
pub use maestro_core::traits::{Memory, SearchResult};
