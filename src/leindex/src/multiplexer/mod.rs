pub mod tmux;
pub mod zellij;

// Re-export the tmux multiplexer as the primary multiplexer
pub use tmux::{TerminalInfo, TmuxMultiplexer, TmuxSession, TmuxSessionStatus};
