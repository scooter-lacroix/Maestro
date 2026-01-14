//! API Module
//!
//! Pure Rust axum-based API server for Maestro.

pub mod server;
pub mod routes;
pub mod handlers;

pub use server::*;
pub use routes::*;
pub use handlers::*;
