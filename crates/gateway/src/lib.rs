//! Maestro Gateway - Web API with SSE/WebSocket streaming
//!
//! This crate provides a web gateway for Maestro, enabling:
//! - WebSocket-based RPC communication
//! - Server-Sent Events (SSE) for real-time updates
//! - REST API for session, MCP, and cron management
//! - Agent execution endpoints for AI assistant sessions
//! - Rate limiting and security middleware
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      maestro-gateway                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │  WebSocket (/ws)  │  SSE (/events)  │  REST API (/api/*)   │
//! ├───────────────────┴─────────────────┴───────────────────────┤
//! │                    Protocol Layer                           │
//! │              (Request/Response/Event Frames)               │
//! ├─────────────────────────────────────────────────────────────┤
//! │                    Gateway State                            │
//! │         (McpManager, SandboxManager, EventBus, Agent)      │
//! ├─────────────────────────────────────────────────────────────┤
//! │                    maestro-core                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use maestro_gateway::{server, state::GatewayConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = GatewayConfig {
//!         port: 8080,
//!         ..Default::default()
//!     };
//!
//!     server::run(config).await
//! }
//! ```

pub mod agent;
pub mod protocol;
pub mod rate_limit;
pub mod routes;
pub mod server;
pub mod sse;
pub mod state;
pub mod ws;

pub use protocol::{EventFrame, RequestFrame, ResponseFrame};
pub use server::run;
pub use state::{GatewayConfig, GatewayState};
pub use agent::{
    AgentExecuteRequest, AgentExecuteResponse, AgentStatusEvent, AgentTurnEvent,
    SessionCreateRequest, SessionDeleteRequest, SessionInfo, SessionListResponse,
    StreamingChunk, ToolCallSummary, ToolExecutionEvent,
};
