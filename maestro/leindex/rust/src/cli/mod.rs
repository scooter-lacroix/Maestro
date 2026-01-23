//! CLI submodules

pub mod analyze;
pub mod implement;
pub mod integrate;
pub mod leindex_cmd;
pub mod mcp;
// Renamed from `memory` to `memory_impl` to avoid shadowing the root `memory` module
pub mod memory_impl;
pub mod memory_cmd;
pub mod orchestrate;
pub mod prompt;
