//! Lattice Models
//!
//! Request and response types for the lattice API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Layer identifier in the analysis lattice
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LatticeLayer {
    /// Layer 1: Abstract Syntax Tree
    Ast,
    /// Layer 2: Call Graph
    CallGraph,
    /// Layer 3: Control Flow Graph
    Cfg,
    /// Layer 4: Data Flow Graph
    Dfg,
    /// Layer 5: Program Slicing
    Slicing,
}

impl LatticeLayer {
    pub const ALL: [LatticeLayer; 5] = [
        LatticeLayer::Ast,
        LatticeLayer::CallGraph,
        LatticeLayer::Cfg,
        LatticeLayer::Dfg,
        LatticeLayer::Slicing,
    ];

    pub fn as_number(&self) -> u8 {
        match self {
            LatticeLayer::Ast => 1,
            LatticeLayer::CallGraph => 2,
            LatticeLayer::Cfg => 3,
            LatticeLayer::Dfg => 4,
            LatticeLayer::Slicing => 5,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            LatticeLayer::Ast => "AST",
            LatticeLayer::CallGraph => "Call Graph",
            LatticeLayer::Cfg => "Control Flow",
            LatticeLayer::Dfg => "Data Flow",
            LatticeLayer::Slicing => "Program Slicing",
        }
    }
}

impl std::fmt::Display for LatticeLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LatticeLayer::Ast => write!(f, "ast"),
            LatticeLayer::CallGraph => write!(f, "call_graph"),
            LatticeLayer::Cfg => write!(f, "cfg"),
            LatticeLayer::Dfg => write!(f, "dfg"),
            LatticeLayer::Slicing => write!(f, "slicing"),
        }
    }
}

impl std::str::FromStr for LatticeLayer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ast" => Ok(LatticeLayer::Ast),
            "call_graph" | "callgraph" => Ok(LatticeLayer::CallGraph),
            "cfg" | "control_flow" => Ok(LatticeLayer::Cfg),
            "dfg" | "data_flow" => Ok(LatticeLayer::Dfg),
            "slicing" | "program_slicing" => Ok(LatticeLayer::Slicing),
            _ => Err(format!(
                "Invalid layer: {}. Must be one of: ast, call_graph, cfg, dfg, slicing",
                s
            )),
        }
    }
}

/// Analysis status for a specific file and layer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnalysisStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// File analysis metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysisMetadata {
    pub file_path: String,
    pub file_hash: String,
    pub language: String,
    pub line_count: usize,
    pub analyzed_at: DateTime<Utc>,
    pub layers: HashMap<String, LayerStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStatus {
    pub layer: LatticeLayer,
    pub status: AnalysisStatus,
    pub analyzed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Complete lattice analysis result for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatticeAnalysisResult {
    pub metadata: FileAnalysisMetadata,
    pub ast: Option<AstResult>,
    pub call_graph: Option<CallGraphResult>,
    pub cfg: Option<CfgResult>,
    pub dfg: Option<DfgResult>,
    pub slicing: Option<SlicingResult>,
}

/// Layer 1: AST result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstResult {
    pub imports: Vec<ImportInfo>,
    pub classes: Vec<ClassInfo>,
    pub functions: Vec<FunctionInfo>,
    pub globals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    pub module: String,
    pub name: Option<String>,
    pub alias: Option<String>,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub line: usize,
    pub bases: Vec<String>,
    pub methods: Vec<FunctionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub line: usize,
    pub args: String,
    pub returns: Option<String>,
    pub is_async: bool,
    pub is_method: bool,
    pub class_name: Option<String>,
}

/// Layer 2: Call Graph result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphResult {
    pub functions: Vec<CallGraphNode>,
    pub edges: Vec<CallEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphNode {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub line: usize,
    pub is_exported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEdge {
    pub from: String,
    pub to: String,
    pub call_count: usize,
}

/// Layer 3: CFG result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfgResult {
    pub functions: Vec<CfgFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfgFunction {
    pub name: String,
    pub complexity: usize,
    pub blocks: Vec<BasicBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicBlock {
    pub id: String,
    pub line_start: usize,
    pub line_end: usize,
    pub successors: Vec<String>,
}

/// Layer 4: DFG result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DfgResult {
    pub functions: Vec<DfgFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DfgFunction {
    pub name: String,
    pub definitions: Vec<Definition>,
    pub uses: Vec<Use>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Definition {
    pub name: String,
    pub line: usize,
    pub var_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Use {
    pub name: String,
    pub line: usize,
    pub definition_line: Option<usize>,
}

/// Layer 5: Slicing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlicingResult {
    pub slices: Vec<ProgramSlice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramSlice {
    pub criterion: SliceCriterion,
    pub lines: Vec<usize>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceCriterion {
    pub line: usize,
    pub variable: Option<String>,
}

/// Request to analyze a file
#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub file_path: String,
    pub layers: Option<Vec<LatticeLayer>>,
    pub force: Option<bool>,
}

/// Request for batch analysis
#[derive(Debug, Deserialize)]
pub struct BatchAnalyzeRequest {
    pub file_paths: Vec<String>,
    pub layers: Option<Vec<LatticeLayer>>,
    pub max_depth: Option<usize>,
}

/// Response for batch analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchAnalyzeResponse {
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub results: Vec<FileAnalysisResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileAnalysisResult {
    pub file_path: String,
    pub status: AnalysisStatus,
    pub layers_completed: Vec<LatticeLayer>,
    pub error: Option<String>,
}

/// Query parameters for lattice search
#[derive(Debug, Deserialize)]
pub struct LatticeSearchQuery {
    pub q: Option<String>,
    pub layer: Option<LatticeLayer>,
    pub language: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
