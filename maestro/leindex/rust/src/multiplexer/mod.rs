pub mod zellij;
pub mod tmux;

// Re-export the tmux multiplexer as the primary multiplexer
pub use tmux::{TmuxMultiplexer, TmuxSession, TmuxSessionStatus, TerminalInfo};
