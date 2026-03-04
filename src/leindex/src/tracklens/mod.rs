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

pub use server::{TrackLensServer, ServerConfig, ReviewContent, ReviewMetadata};
pub use types::*;
pub use walkthrough::{WalkthroughGenerator, WalkthroughConfig};

// ─── Module Documentation ─────────────────────────────────────────────────────

/// TrackLens provides visual review, annotation, and walkthrough capabilities
/// for Maestro tracks. This module integrates with LeIndex to provide:
///
/// - Code-aware walkthroughs with LeIndex analysis
/// - Annotation storage with code references
/// - Browser-based review UI integration
///
/// # Status
///
/// 🚧 **Under Construction** - Porting from Plannotator to TrackLens
///
/// # Architecture
///
/// ```text
/// tracklens/
/// ├── types.rs       - Core types (modes, decisions, annotations)
/// ├── server.rs      - Axum server for review UI
/// └── walkthrough.rs - Generator for track walkthroughs
/// ```
pub mod tracklens {}
