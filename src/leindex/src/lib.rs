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
pub use token_format::*;

pub const MAX_FILE_SIZE: usize = 1048576; // 1MB
