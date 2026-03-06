//! System Portability Module
//!
//! Provides cross-platform utilities for:
//! - XDG Base Directory specification compliance
//! - Executable discovery via PATH
//! - Runtime distro detection and package manager abstraction
//! - Path resolution without hardcoded absolute paths
//!
//! ## Architecture
//!
//! This module is designed to work on Arch-based systems (CachyOS), Debian-based
//! systems, Fedora, and macOS without modification.
//!
//! ## Usage
//!
//! ```no_run
//! use maestro_core::portability::{find_executable, xdg_dirs::data_home};
//!
//! // Find an executable in PATH
//! if let Some(path) = find_executable("pi") {
//!     println!("Found pi at: {:?}", path);
//! }
//!
//! // Get XDG-compliant data directory
//! let data_dir = data_home();
//! println!("Data directory: {:?}", data_dir);
//! ```

pub mod executable;
pub mod paths;
pub mod xdg_dirs;

pub use executable::*;
pub use paths::*;
pub use xdg_dirs::*;
