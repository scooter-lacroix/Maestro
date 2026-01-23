//! API Module
//!
//! Pure Rust axum-based API server for Maestro.
//!
//! **NOTE:** This module is only available when the "rusqlite" feature is enabled.
//! The API depends on the legacy MemoryService which uses rusqlite.

#[cfg(feature = "rusqlite")]
pub mod handlers;
#[cfg(feature = "rusqlite")]
pub mod routes;
#[cfg(feature = "rusqlite")]
pub mod server;

#[cfg(feature = "rusqlite")]
pub use handlers::*;
#[cfg(feature = "rusqlite")]
pub use routes::*;
#[cfg(feature = "rusqlite")]
pub use server::*;
