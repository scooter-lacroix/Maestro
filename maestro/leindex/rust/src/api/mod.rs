//! API Module
//!
//! Pure Rust axum-based API server for Maestro.
//!
//! **NOTE:** This module is only available when the "rusqlite" feature is enabled.
//! The API depends on the legacy MemoryService which uses rusqlite.

// Shared response types (always available)
pub mod response;

#[cfg(feature = "rusqlite")]
pub mod handlers;
#[cfg(feature = "rusqlite")]
pub mod routes;
#[cfg(feature = "rusqlite")]
pub mod server;

// Lattice module is always available (doesn't depend on rusqlite)
pub mod lattice;

// Re-export shared response types
pub use response::ApiResponse;

#[cfg(feature = "rusqlite")]
pub use handlers::*;
#[cfg(feature = "rusqlite")]
pub use server::*;

// Re-export specific lattice items
pub use lattice::models::*;
// Re-export specific lattice handler items (not ApiResponse to avoid ambiguity)
pub use lattice::handlers::{LatticeAppState, LatticeService};
