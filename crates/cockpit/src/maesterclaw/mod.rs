//! MaesterClaw command center modules
//!
//! This module provides the command center functionality for the MaesterClaw tab,
//! including channel management, gateway connections, and hot cache for suggestions.

pub mod agent_status;
pub mod channels;
pub mod gateway;
pub mod hot_cache;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_integration_tests;

pub use agent_status::{AgentStatus, SessionDisplay, TurnDisplay};
pub use channels::{ChannelConfig, ChannelControlPlane, ChannelStatus, ChannelType};
pub use gateway::{ConnectedClient, GatewayAuthStatus, GatewayConfig, GatewayControlPlane};
pub use hot_cache::{clamp_flash, BufferedSuggestion, HotCache, MemorySuggestion, SuggestionTtl};
