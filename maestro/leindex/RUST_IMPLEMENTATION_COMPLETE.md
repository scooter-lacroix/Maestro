# LeIndex Rust Implementation - MAXIMALLY DETAILED PLAN

## Objective

Port TLDR's 5-layer analyzers from Python to **pure Rust**, then expose via PyO3 to Python.

**Requirements:**
- NO STUBS - Every function must be fully implemented
- NO SHIMS - Direct Rust implementation
- NO PYTHON FALLBACK - All analysis logic in Rust
- Token-efficient output from Rust (no Python strings mixed in)
- Preserve 95%+ token savings while keeping code actionable

## Architecture

```
Python Source → (Python's ast module) → JSON → Rust Analysis → Token-Efficient String
```

The key insight: We use Python's ast module via PyO3 for parsing, but ALL analysis logic is in Rust.

---

## File Structure

```
maestro/leindex/rust/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Main module, PyO3 bindings
│   ├── ast.rs              # AST Analyzer (Layer 1) - COMPLETE IMPLEMENTATION
│   ├── callgraph.rs        # Call Graph (Layer 2) - COMPLETE IMPLEMENTATION
│   ├── cfg.rs              # CFG Analyzer (Layer 3) - COMPLETE IMPLEMENTATION
│   ├── dfg.rs              # DFG Analyzer (Layer 4) - COMPLETE IMPLEMENTATION
│   ├── slicing.rs          # Slicing (Layer 5) - COMPLETE IMPLEMENTATION
│   ├── token_format.rs      # Token-efficient formatting
│   └── utils.rs             # Utility functions
├── src/ast/                # AST sub-module
├── src/cfg/                # CFG sub-module
├── src/cfg/                # CFG sub-module
├── src/dfg/                # DFG sub-module
├── src/slicing/            # Slicing sub-module
└── build.rs                # Build script
```

---

## Implementation Details

### Layer 1: AST Analysis (ast.rs)

**Purpose:** Extract code structure without implementation details

**Data Structures:**
- `ASTAnalysis` - Complete file structure
- `FunctionInfo` - Function signature details
- `ClassInfo` - Class hierarchy
- `ImportInfo` - Import statement details

**Key Functions:**
- `analyze(source: &str, path: &str) -> ASTAnalysis`
- `to_llm_string(&self) -> String`

**Rust Implementation Requirements:**
1. Parse Python source line-by-line for:
   - Class definitions (`class Name(Base1, Base2):`)
   - Function definitions (`def name(args) -> ret:`)
   - Imports (`from x import y`, `import x`)
   - Async markers (`async def`)
   - Docstrings (truncated to 80 chars)
2. Build hierarchical structure tracking:
   - Classes with their methods
   - Functions with their decorators and metadata
3. Output token-efficient string

### Layer 2: Call Graph (callgraph.rs)

**Purpose:** Cross-file function call relationships

**Data Structures:**
- `CallGraph` - Complete graph with nodes, edges
- `CallNode` - Function with callers/callees
- `CallEdge` - Directed edge with call type
- `ImpactAnalysis` - Results of impact analysis

**Key Functions:**
- `build_project_graph(project_path: &str) -> CallGraph`
- `build_file_graph(path: &str, functions: &[FunctionInfo]) -> CallGraph`
- `get_callers(function: &str) -> Vec<&CallNode>`
- `get_callees(function: &str) -> Vec<&CallNode>`

**Rust Implementation Requirements:**
1. Parse function calls from function bodies:
   - `function_name(args)` pattern matching
   - `obj.method()` calls (if class info available)
2. Build adjacency list for each function
3. Build cross-file relationships via imports
4. Find entry points (main, __init__, etc.)
5. Output: list of callers/callees per function

### Layer 3: CFG Analysis (cfg.rs)

**Purpose:** Control flow complexity analysis

**Data Structures:**
- `ControlFlowGraph` - CFG with nodes and edges
- `CFGNode` - Basic block, condition, loop nodes
- `CFGEdge` - Edge type (true_branch, false_branch, fallthrough)
- `ComplexityMetrics` - Cyclomatic complexity, nesting depth, etc.

**Key Functions:**
- `analyze_function(source: &str, function: &str) -> ControlFlowGraph`
- `build_cfg(function_body: &[ast::Stmt], entry_node: &str) -> ControlFlowGraph`
- `calculate_metrics(&cfg) -> ComplexityMetrics`
- `find_paths(&cfg) -> Vec<Vec<String>>`
- `get_complexity(&cfg) -> ComplexityMetrics`

**Rust Implementation Requirements:**
1. Process AST statement-by-statement:
   - `if/elif/else` → conditional nodes
   - `for/while` → loop nodes with back-edges
   - `try/except` → exception handling
   - `return` → exit nodes
2. Track node predecessors and successors
3. Calculate cyclomatic complexity
4. Identify nesting depth
5. Format to LLM string with complexity metrics

### Layer 4: DFG Analysis (dfg.rs)

**Purpose:** Data flow - variable definitions and uses

**Data Structures:**
- `DataFlowGraph` - DFG with variables and edges
- `VariableInfo` - Variable with definition and uses
- `DataFlowNode` - Node with variable access
- `DataFlowEdge` - Data dependence edge

**Key Functions:**
- `analyze_function(source: &str, function: &str) -> DataFlowGraph`
- `build_dfg(function_body: &[ast::Stmt]) -> DataFlowGraph`
- `get_data_dependencies(variable: &str) -> Vec<String>`
- `slice_backward(source: &str, function: &str, line: usize) -> Vec<(usize, String)>`

**Rust Implementation Requirements:**
1. Visit AST tracking Name nodes
2. Classify Name ctx:
   - Load → VarAction::READ
   - Store → VarAction::DEFINE or MODIFY if already defined
3. Track definitions and uses for each variable
4. Build def-use chains
5. Output: list of (line, variable) tuples for slice

### Layer 5: Slicing (slicing.rs)

**Purpose:** Program dependence analysis and slicing

**Data Structures:**
- `ProgramDependenceGraph` - Combined CFG + DFG
- `SliceResult` - Relevant lines for a target line
- `SliceDirection` - Backward, Forward, Both
- `PDGNode` - Node with line number and statement
- `PDGEdge` - Edge with type (control or data)

**Key Functions:**
- `build_pdg(source: &str, function: &str) -> ProgramDependenceGraph`
- `slice_backward(source: &str, function: &str, line: usize) -> SliceResult`
- `slice_forward(source: &str, function:Str: str, line: usize) -> SliceResult`
- `compute_chop(source: &str, function: &str, from_line: usize, to_line: usize) -> Vec<usize>`

**Rust Implementation Requirements:**
1. Combine CFG and DFG edges into PDG
2. Add edges as (from_line, to_line, edge_type)
3. BFS for backward slice (find all statements that influence a line)
4. BFS for forward slice (find all statements influenced by a line)
5. Output: list of line numbers and involved variables

---

## Implementation Order

### Phase 1: Setup & AST Layer

**Files:**
1. `utils.rs` - Utility functions
2. `token_format.rs` - Token-efficient string formatting
3. `ast.rs` - Complete AST analyzer (no stubs)
4. `lib.rs` - Update bindings

**Acceptance Criteria:**
- All AST functions fully implemented in Rust
- Can parse Python classes and functions accurately
- Token-efficient output matches TLDR's format
- Can be imported in Python and used

### Phase 2: Call Graph Layer

**Files:**
1. `callgraph.rs` - Complete call graph analyzer (no stubs)

**Acceptance Criteria:**
- Can trace function calls accurately
- Builds cross-file relationships
- Finds entry points automatically
- Returns caller/callee relationships

### Phase 3: CFG Layer

**Files:**
1. `cfg.rs` - Complete CFG analyzer (no stubs)

**Acceptance Criteria:**
- Accurately tracks control flow
- Calculates cyclomatic complexity
- Identifies all branches, loops, exceptions
- Returns execution paths

### Phase 4: DFG Layer

**Files:**
1. `dfg.rs` - Complete DFG analyzer (no stubs)

**Acceptance Criteria:**
- Tracks variable definitions and uses
- Can trace data dependencies
- Def-use chains for each variable
- Slice backward using def-use chains

### Phase 5: Slicing Layer

**Files:**
1. `slicing.rs` - Complete slicing analyzer (no stubs)

**Acceptance:**
- Can combine CFG and DFG into PDG
- Backward slice finds all statements influencing a line
- Forward slice finds all statements influenced by a line
- CHOP (slice between two points) works correctly

---

## Python Bridge (lib.rs - PyO3 bindings)

**Functions to expose via PyO3:**

```python
# Main analyzer
class RustASTAnalyzer:
    def analyze(source: str, path: str) -> ASTAnalysis

# Context extractor
class RustContextExtractor:
    def extract_for_file(file_path: str) -> ContextExtractionResult

# Analyzers
class RustCallGraphAnalyzer:
    def build_project_graph(project_path: str) -> CallGraph

class RustCFGAnalyzer:
    def analyze_function(source: str, function_name: str) -> ControlFlowGraph

class RustDFGAnalyzer:
    def analyze_function(source: str, function_name: str) -> DataFlowGraph

class RustSlicingAnalyzer:
    def slice_backward(source: str, function_name: str, line: int) -> SliceResult
```

---

## Implementation Checklist

### Phase 1: AST Layer (ast.rs)

- [ ] Parse class definitions correctly
- [ ] Parse function definitions correctly (async/def)
- ] Parse imports (from x import y, import x)
- ] Extract function signatures with:
  - Arguments (condensed to 15 chars per arg max)
  - Return types (condensed to 20 chars max)
  - Async marker (prefix with 'a')
- [ ] Track line numbers for navigation
- ] Extract class hierarchies with:
  - Base classes (condensed)
  - Methods (with condensed signatures)
- [ ] Output token-efficient string matching TLDR's format
- [ ] Handle docstrings (truncated to 80 chars)
- [ ] Track decorators (async, property, etc.)
- [ ] Handle class methods properly

### Phase 2: Call Graph (callgraph.rs)

- [ ] Extract function calls from function bodies
- ] Handle `function(args)` patterns
- ] Handle `obj.method()` patterns (if class info available)
- ] Build adjacency list for each function
- ] Build cross-file relationships via import mapping
- ] Find entry points (main, __init__, test_*, etc.)
- ] Output: caller/callee relationships per function
- ] Include line numbers for navigation
- ] Format to LLM-friendly string

### Phase 3: CFG (cfg.rs)

- [ ] Create node for each basic block
- ] Create condition nodes for if/elif/else
- ] Create loop nodes for for/while
- ] Create exit nodes for return
- ] Add edges between nodes
- ] Track predecessors and successors
- ] Calculate cyclomatic complexity correctly
- ] Calculate max nesting depth
- ] Identify all branches, loops, exceptions
- ] Format to LLM-friendly string with metrics

### Phase 4: DFG (dfg.rs)

- [ ] Track all Name nodes in AST
- ] Classify Load vs Store operations
- ] Track definitions per variable
- ] Track uses per variable
- ] Build def-use chains per variable
- ] Handle augmented assignment (+=, -=, etc.)
- ] Handle for loop variables
- ] Handle comprehensions
- ] Output: line numbers and variables involved

### Phase 5: Slicing (slicing.rs)

- [ ] Combine CFG and DFG edges into PDG
- ] Add control edges: (from_line, to_line, "control")
- ] Add data edges: (from_line, to_line, "data")
- ] BFS backward: start at target, follow predecessors
- ] BFS forward: start at target, follow successors
- **CRITICAL:** NO STUBS - every function must be implemented
- **CRITICAL:** NO PYTHON FALLBACKS - all logic in Rust
- **CRITICAL:** Token output comes from Rust, not Python

---

## Key Design Decisions

### 1. Python AST Bridge vs Pure Rust Parser

**Decision:** Use Python's ast module via PyO3 bridge

**Reasoning:**
- Python's ast module is battle-tested for Python parsing
- Writing a Python parser from scratch is a months-long project
- AST parsing is only ~500 tokens out of 10,000+, not the bottleneck
- The 94%+ token savings comes from structure extraction, not parsing

**Trade-off:** Small overhead of calling Python's ast module, but correct parsing is worth it.

### 2. Token Format

**Decision:** Replicate TLDR's exact format to ensure compatibility

**Format:**
```
File: path/to/file.py

# Imports
from x import y

# Classes
class Name(Base1, Base2):
    def method_name(arg1, arg2)

# Functions
def function_name(arg1, arg2) -> returnType
```

### 3. Error Handling

**Decision:** Use `anyhow::Result<T>` for all functions

**Reasoning:**
- Provides descriptive error messages
- Easy to integrate with error-chain
- Can be converted to PyO3 exceptions

### 4. Data Structures

**Decision:** Use `serde::{Serialize, Deserialize}` for all data structures

**Reasoning:**
- Easy serialization to JSON for debugging
- Can create PyO3 bindings automatically
- Clean separation of data and logic

---

## Testing Strategy

1. **Unit Tests** in Rust:
   - Test each analyzer in isolation
   - Test with known Python code snippets
   - Verify token-efficiency output matches expected

2. **Integration Tests:**
   - Test the PyO3 bindings work from Python
   - Verify LLM can use the output

3. **Comparison Tests:**
   - Compare output with Python TLDR output
   - Ensure token savings match or exceed original TLDR

---

## Timeline Estimate

| Phase | Estimated Time |
|-------|----------------|
| Phase 1 (AST) | 4-6 hours |
| Phase 2 (Call Graph) | 2-3 hours |
| Phase 3 (CFG) | 3-4 hours |
| Phase 4 (Dfg) | 3-4 hours |
| Phase 5 (Slicing) | 4-5 hours |
| PyO3 Bindings | 2-3 hours |
| Testing | 2-3 hours |
| **Total** | **20-25 hours** |

---

## Deliverables

When complete, we will have:

1. **Complete Rust implementations** of all 5 layers
2. **PyO3 module** (`leindex_analyzers.so` Python extension)
3. **Updated Python wrapper** that uses the Rust backend
4. **Test suite** validating the implementation
5. **Documentation** explaining the architecture

The system will be:
- **Pure Rust analysis** (all logic in Rust, no Python fallback)
- **Token-efficient output** directly from Rust
- **Exposed to Python** via PyO3
- **Integrated into LeIndex** module
- **Ready for installation** via pip/maturin

---

## Installation

When complete, installation will be:

```bash
# Install Rust dependencies
cd maestro/leindex/rust
cargo build --release
pip install maturin
maturin develop --release
```

The Python wrapper will detect and use the compiled Rust module automatically.

---

## Summary

This plan provides a complete roadmap to implement TLDR in Rust with NO STUBS, ensuring:
- Complete Rust implementation of all 5 analyzers
- PyO3 integration to Python
- Token-efficient output from Rust
- 95%+ token savings with semantic completeness
- Production-ready Rust code with no Python fallbacks

The key is that EVERY function in the plan must be FULLY IMPLEMENTED in Rust - no stubs, no shims, no fallbacks.
