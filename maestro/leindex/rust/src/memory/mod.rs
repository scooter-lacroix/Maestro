//! Memory Module
//!
//! Pure Rust implementation of the Maestro Memory System.
//! Provides database, scanner, and service functionality.

pub mod lsp_manager;
pub mod lsp_pool;
pub mod mcp_discovery;
pub mod models;
pub mod scanner;
pub mod schema;
pub mod search;
pub mod turso_backend;

// Legacy rusqlite-based modules (only available with "rusqlite" feature)
#[cfg(feature = "rusqlite")]
pub mod db;
#[cfg(feature = "rusqlite")]
pub mod mcp_pool;
#[cfg(feature = "rusqlite")]
pub mod migration;
#[cfg(feature = "rusqlite")]
pub mod service;
#[cfg(feature = "rusqlite")]
pub mod session_manager;

pub use lsp_manager::*;
pub use lsp_pool::*;
pub use mcp_discovery::*;
pub use models::*;
pub use scanner::*;
pub use schema::*;
pub use search::*;
// Note: turso_backend::* not re-exported due to name conflict with models::MemoryCategoryStats
// Use `use leindex_core::memory::turso_backend::X` when needed

// Legacy rusqlite-based re-exports (only available with "rusqlite" feature)
#[cfg(feature = "rusqlite")]
pub use db::*;
#[cfg(feature = "rusqlite")]
pub use mcp_pool::*;
#[cfg(feature = "rusqlite")]
pub use migration::*;
#[cfg(feature = "rusqlite")]
pub use service::*;
#[cfg(feature = "rusqlite")]
pub use session_manager::*;
