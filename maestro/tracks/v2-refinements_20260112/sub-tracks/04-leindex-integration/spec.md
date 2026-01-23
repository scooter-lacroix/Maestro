# Sub-Track 04: LeIndex Integration - Specification

## Overview

Build LeIndex as Maestro's unified search and code analysis system, incorporating TLDR's 5-layer analysis capabilities with LeIndexer's search infrastructure.

**Priority:** 4 (Search & Analysis)
**Parent Track:** v2-refinements_20260112

## Functional Requirements

### FR-1: Module Structure
- Create `maestro/leindex/` as core module (installs with Maestro)
- Clean public API via `__init__.py`
- Well-defined interfaces for each component

### FR-2: 5-Layer Static Analysis (from TLDR)
- **AST Analyzer:** Function signatures, imports, class definitions
- **Call Graph Analyzer:** Cross-file function relationships
- **CFG Analyzer:** Control flow, cyclomatic complexity
- **DFG Analyzer:** Variable definitions, uses, modifications
- **Slicing Analyzer:** Program dependence graph, backward/forward slicing
- All analyzers implement `to_llm_string()` for token-efficient output

### FR-3: Tantivy Search Backend
- Full-text indexing with BM25 ranking
- Code-aware tokenization
- Fast index creation and updates

### FR-4: LEANN Vector Store
- Code embeddings storage
- Semantic similarity search
- Efficient vector indexing and retrieval

### FR-5: Hybrid Ranker
- Combine semantic + BM25 + recency scores
- Configurable weight parameters
- Cross-project ranking support

### FR-6: MCP Server
- Native MCP protocol implementation
- Search and analysis tool endpoints
- Integration with Maestro's MCP configuration

### FR-7: Daemon Mode
- Background indexing with file watcher
- Incremental index updates
- Lifecycle management

### FR-8: Memory Integration
- Connect to Maestro memory system
- Store code entities as Memory objects
- Session context for temporal analysis

## New Files

```
maestro/leindex/
├── __init__.py           # Public API
├── analyzers/
│   ├── ast.py
│   ├── callgraph.py
│   ├── cfg.py
│   ├── dfg.py
│   └── slicing.py
├── search/
│   ├── tantivy_backend.py
│   ├── leann_store.py
│   └── hybrid_ranker.py
├── mcp_server.py
├── daemon.py
└── memory_integration.py
```

## Acceptance Criteria

1. [ ] All 5 analyzers implemented with `to_llm_string()`
2. [ ] Tantivy full-text search functional
3. [ ] LEANN vector store functional
4. [ ] Hybrid ranker combines all signals
5. [ ] MCP server exposes search/analysis tools
6. [ ] Daemon mode indexes in background
7. [ ] Memory integration stores code entities
8. [ ] All tests passing with >98% coverage
9. [ ] Tzar of Excellence review approved

## Out of Scope

- Standalone LeIndex package compatibility (separate project)
- GPU-accelerated embeddings
- Remote/distributed indexing
