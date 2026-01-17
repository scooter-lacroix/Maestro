//! Memory Module
//!
//! Pure Rust implementation of the Maestro Memory System.
//! Provides database, scanner, and service functionality.

pub mod models;
pub mod schema;
pub mod db;
pub mod scanner;
pub mod service;
pub mod search;
pub mod migration;
pub mod session_manager;
pub mod mcp_pool;
pub mod mcp_discovery;

pub use models::*;
pub use schema::*;
pub use db::*;
pub use scanner::*;
pub use service::*;
pub use search::*;
pub use mcp_discovery::*;
pub use mcp_pool::*;
