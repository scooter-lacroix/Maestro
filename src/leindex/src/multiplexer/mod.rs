pub mod maestro_tab;
pub mod tmux;
pub mod zellij;

// Re-export the tmux multiplexer types (implementation details)
pub use tmux::{
    TerminalInfo, TmuxMultiplexer as TmuxMultiplexerImpl, TmuxSession as TmuxSessionImpl,
    TmuxSessionStatus as TmuxSessionStatusImpl,
};

// Re-export helper functions from tmux module
pub use tmux::{sanitize_name, shell_quote};

// Feature-gated multiplexer selection
// Default: Use MaestroTabMultiplexer (delegates to tmux with tab-rs integration hooks)
// tmux-only: Use TmuxMultiplexer directly (for rollback)
#[cfg(feature = "maestro-tab")]
pub use maestro_tab::{MaestroTabMultiplexer, MaestroTabSession, MaestroTabSessionStatus};

#[cfg(feature = "maestro-tab")]
pub type TmuxMultiplexer = MaestroTabMultiplexer;

#[cfg(feature = "maestro-tab")]
pub type TmuxSession = MaestroTabSession;

#[cfg(feature = "maestro-tab")]
pub type TmuxSessionStatus = MaestroTabSessionStatus;

// tmux-only mode: Direct use of TmuxMultiplexer
#[cfg(not(feature = "maestro-tab"))]
pub type TmuxMultiplexer = tmux::TmuxMultiplexer;

#[cfg(not(feature = "maestro-tab"))]
pub type TmuxSession = tmux::TmuxSession;

#[cfg(not(feature = "maestro-tab"))]
pub type TmuxSessionStatus = tmux::TmuxSessionStatus;
