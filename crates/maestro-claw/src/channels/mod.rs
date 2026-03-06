//! MaestroClaw channel system.

#[cfg(feature = "channels")]
pub mod discord;
pub mod dispatcher;
#[cfg(feature = "channels")]
pub mod slack;
#[cfg(feature = "channels")]
pub mod telegram;
pub mod traits;

pub use dispatcher::ChannelDispatcher;
pub use traits::{Channel, ChannelMessage, SendMessage};
