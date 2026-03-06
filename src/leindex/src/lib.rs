//! LeIndex Analyzers - Pure Rust Code Analysis
//!
//! High-performance 5-layer code analysis using tree-sitter.
//! Supports 8 programming languages.
//!
//! ## Layers
//!
//! - Layer 1: AST - Function signatures, imports, classes
//! - Layer 2: Call Graph - Function relationships
//! - Layer 3: CFG - Control flow complexity
//! - Layer 4: DFG - Data flow analysis
//! - Layer 5: Slicing - Program dependence

pub mod api;
pub mod ast_analyzer;
pub mod callgraph;
pub mod cfg;
pub mod cli;
pub mod config;
pub mod dfg;
pub mod five_phase;
pub mod language;
pub mod lsp;
pub mod memory;
pub mod migrations;
pub mod multi_lang_ast;
pub mod multi_lang_callgraph;
pub mod multi_lang_cfg;
pub mod multi_lang_dfg;
pub mod multi_lang_slicing;
pub mod multiplexer;
pub mod orchestrate;
pub mod setup;
pub mod slicing;
pub mod token_format;
pub mod tracklens;
pub mod vector;

// Re-export commonly used submodules for convenient access
pub use lsp::*;
// Re-export vector module contents but also keep vector module accessible for submodules like vector::report
pub use vector::*;

pub use ast_analyzer::*;
pub use callgraph::*;
pub use cfg::*;
pub use dfg::{FunctionDataFlow as PythonFunctionDataFlow, *};
pub use language::*;
pub use multi_lang_ast::*;
pub use multi_lang_callgraph::*;
pub use multi_lang_cfg::*;
pub use multi_lang_dfg::{
    FunctionDataFlow as MultiLangFunctionDataFlow, MultiLangDFGAnalyzer, MultiLangDFGResult,
};
pub use multi_lang_slicing::*;
pub use orchestrate::*;
pub use slicing::*;

// Re-export config module's Config explicitly before setup to avoid shadowing
pub use config::Config as AppConfig;

// Re-export setup module (includes its own Config)
pub use setup::*;

// Re-export commonly used modules for convenient crate:: access
pub use multiplexer::*;

// Explicit re-exports for commonly used analyzer types
pub use multi_lang_ast::MultiLangASTAnalyzer;
pub use multi_lang_callgraph::MultiLangCallGraphAnalyzer;
pub use multi_lang_cfg::MultiLangCFGAnalyzer;
// MultiLangDFGAnalyzer is already imported via wildcard above
pub use multi_lang_slicing::MultiLangSlicingAnalyzer;
pub use language::ProgrammingLanguage;

// Explicit re-exports for commonly used types
pub use multiplexer::TmuxMultiplexer;
pub use token_format::TokenFormatter;

// Re-export memory module items selectively to avoid ambiguity with orchestrate module
// Both orchestrate and memory export SessionStatus and TrackStatus, so we use explicit imports
pub use memory::lsp_manager::*;
pub use memory::mcp_discovery::*;
pub use memory::scanner::*;
pub use memory::schema::*;
pub use memory::search::*;
pub use memory::turso_backend::*;

// Explicit re-exports for commonly used memory types
pub use memory::models::Session as MemorySession;
pub use memory::models::SessionGroup;
pub use memory::models::SessionStatus as MemorySessionStatus;
pub use memory::models::McpServer;
pub use memory::models::McpStatus;
pub use memory::models::Memory;
pub use memory::models::MemoryCategory;
pub use memory::models::MemoryImportance;
pub use memory::lsp_manager::LspType;
pub use memory::turso_backend::LspStatus;
pub use memory::turso_backend::TursoStorageBackend;
#[cfg(feature = "rusqlite")]
pub use memory::session_manager::SessionManager;
#[cfg(feature = "rusqlite")]
pub use memory::session_manager::SessionRestoreMode;
#[cfg(feature = "rusqlite")]
pub use memory::service::MemoryService;
#[cfg(feature = "rusqlite")]
pub use memory::mcp_pool::McpPool;

pub const MAX_FILE_SIZE: usize = 1048576; // 1MB
