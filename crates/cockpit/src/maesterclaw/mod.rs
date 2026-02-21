//! MaesterClaw command center and setup wizard modules
//!
//! This module provides the setup wizard readiness checking and
//! command center functionality for the MaesterClaw tab.

pub mod channels;
pub mod gateway;
pub mod hot_cache;
pub mod readiness;
#[cfg(test)]
mod tests;

pub use channels::{ChannelConfig, ChannelControlPlane, ChannelStatus, ChannelType};
pub use gateway::{ConnectedClient, GatewayAuthStatus, GatewayConfig, GatewayControlPlane};
pub use hot_cache::{
    BufferedSuggestion, clamp_flash, HotCache, MemorySuggestion, SuggestionTtl,
};
pub use readiness::{evaluate_readiness, is_setup_complete, update_step_readiness, ReadinessResult};
