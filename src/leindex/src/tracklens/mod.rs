// TrackLens Module - LeIndex Integration
//
// This module provides TrackLens functionality for LeIndex:
// - Walkthrough generation from track metadata
// - Annotation storage and retrieval
// - Integration with LeIndex code analysis

// ─── Submodules ─────────────────────────────────────────────────────────────

pub mod server;
pub mod types;
pub mod walkthrough;

// ─── Re-exports ───────────────────────────────────────────────────────────────

pub use server::{ReviewContent, ReviewMetadata, ServerConfig, TrackLensServer};
pub use types::*;
pub use walkthrough::{WalkthroughConfig, WalkthroughGenerator};
