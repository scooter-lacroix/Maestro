pub mod tmux;
pub mod zellij;
pub mod maestro_tab;

// Re-export the tmux multiplexer types (implementation details)
pub use tmux::{TerminalInfo, TmuxMultiplexer as TmuxMultiplexerImpl, TmuxSession as TmuxSessionImpl, TmuxSessionStatus as TmuxSessionStatusImpl};

// Re-export MaestroTab multiplexer as the primary multiplexer
// This provides a compatibility layer that will eventually use tab-rs
pub use maestro_tab::{MaestroTabMultiplexer, MaestroTabSession, MaestroTabSessionStatus};

// Re-export helper functions from tmux module
pub use tmux::{sanitize_name, shell_quote};

// Type aliases for backward compatibility
// These allow existing code to work with minimal changes
pub type TmuxMultiplexer = MaestroTabMultiplexer;
pub type TmuxSession = MaestroTabSession;
pub type TmuxSessionStatus = MaestroTabSessionStatus;
