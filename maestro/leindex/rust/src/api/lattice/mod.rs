//! Lattice API Module
//!
//! API endpoints for querying the 5-layer code analysis lattice.
//! The lattice consists of:
//! - Layer 1: AST - Function signatures, imports, classes
//! - Layer 2: Call Graph - Function relationships
//! - Layer 3: CFG - Control flow complexity
//! - Layer 4: DFG - Data flow analysis
//! - Layer 5: Slicing - Program dependence

pub mod handlers;
pub mod models;
pub mod routes;

pub use handlers::*;
pub use models::*;
