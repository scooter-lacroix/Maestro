//! Memory Module
//!
//! Pure Rust implementation of the Maestro Memory System.
//! Provides database, scanner, and service functionality.

pub mod models;
pub mod schema;
pub mod scanner;
pub mod search;
pub mod mcp_discovery;
pub mod turso_backend;
pub mod lsp_manager;

// Legacy rusqlite-based modules (only available with "rusqlite" feature)
#[cfg(feature = "rusqlite")]
pub mod db;
#[cfg(feature = "rusqlite")]
pub mod service;
#[cfg(feature = "rusqlite")]
pub mod migration;
#[cfg(feature = "rusqlite")]
pub mod session_manager;
#[cfg(feature = "rusqlite")]
pub mod mcp_pool;

pub use models::*;
pub use schema::*;
pub use scanner::*;
pub use search::*;
pub use mcp_discovery::*;
pub use turso_backend::*;
pub use lsp_manager::*;

// Legacy rusqlite-based re-exports (only available with "rusqlite" feature)
#[cfg(feature = "rusqlite")]
pub use db::*;
#[cfg(feature = "rusqlite")]
pub use service::*;
#[cfg(feature = "rusqlite")]
pub use migration::*;
#[cfg(feature = "rusqlite")]
pub use session_manager::*;
#[cfg(feature = "rusqlite")]
pub use mcp_pool::*;
