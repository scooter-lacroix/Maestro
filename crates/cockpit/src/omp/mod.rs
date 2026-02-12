//! OMP (oh-my-pi) Integration Module
//!
//! Provides first-class tool provider integration for oh-my-pi within Maestro Cockpit.
//! OMP runs as a managed subprocess with IPC communication for tool invocation.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     JSON-RPC      ┌─────────────────┐
//! │  Cockpit TUI    │  ───────────────▶ │   OMP Worker    │
//! │  (Rust)         │                    │   (Bun/TS)      │
//! │                 │  ◀───────────────  │                 │
//! └─────────────────┘     stdout/SSE     └─────────────────┘
//! ```
//!
//! ## Features
//!
//! - Python execution via IPython kernel
//! - Patch-based editing (safer than sed)
//! - WASM-accelerated grep/find
//! - Session compaction and summarization

mod bridge;
pub mod protocol;
mod provider;
mod worker;

pub use bridge::OmpBridge;
pub use protocol::{OmpRequest, OmpResponse, OmpToolResult};
pub use provider::OmpToolProvider;
pub use worker::{OmpWorker, OmpWorkerConfig};
pub use protocol::OmpWorkerStatus;
