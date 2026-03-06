//! Capabilities Module
//!
//! This module implements Phase 3 capabilities for the Maestro Overhaul:
//! - Sub-agent delegation (`spawn_agent` tool)
//! - Routines engine (cron & event scheduling)
//! - Sandboxing (WASM + Docker)
//! - MCP client integration
//!
//! Based on patterns from IronClaw, ZeroClaw, and Moltis as documented in
//! `maestro/tracks/overhaul_20260217/phase3-5_guidance.md`.

pub mod cron;
pub mod delegate;
pub mod mcp;
pub mod sandbox;

pub use cron::*;
pub use delegate::*;
pub use mcp::*;
pub use sandbox::*;
