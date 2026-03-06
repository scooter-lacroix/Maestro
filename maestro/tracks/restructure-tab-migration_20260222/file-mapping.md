# File Mapping: LeIndex Restructure

**Track:** restructure-tab-migration_20260222
**Phase:** 1 - Project Restructuring
**Created:** 2026-02-22

---

## Overview

This document maps all files from the current `maestro/leindex/rust/src/` structure to the new `src/leindex/` structure.

**Total Files:** 93 Rust source files
**Total Directories:** 12 main directories

---

## Directory Structure Mapping

### Current Structure
```
maestro/leindex/rust/
├── Cargo.toml
├── src/
│   ├── api/
│   ├── bench/
│   ├── cli/
│   ├── lsp/
│   ├── memory/
│   ├── migrations/
│   ├── multiplexer/
│   ├── orchestrate/
│   ├── setup/
│   ├── vector/
│   └── [root files]
└── bin/
```

### Target Structure
```
src/leindex/
├── Cargo.toml
├── src/
│   ├── api/
│   ├── bench/
│   ├── cli/
│   ├── lsp/
│   ├── memory/
│   ├── migrations/
│   ├── multiplexer/
│   ├── orchestrate/
│   ├── setup/
│   ├── vector/
│   └── [root files]
└── bin/
```

---

## Detailed File Mapping (93 Files)

### Root Level Files (17 files)

| Current Path | New Path | Notes |
|--------------|----------|-------|
| `maestro/leindex/rust/src/lib.rs` | `src/leindex/src/lib.rs` | Main library entry point |
| `maestro/leindex/rust/src/main.rs` | `src/leindex/src/main.rs` | Binary entry point |
| `maestro/leindex/rust/src/config.rs` | `src/leindex/src/config.rs` | Configuration module |
| `maestro/leindex/rust/src/language.rs` | `src/leindex/src/language.rs` | Language definitions |
| `maestro/leindex/rust/src/token_format.rs` | `src/leindex/src/token_format.rs` | Token formatting |
| `maestro/leindex/rust/src/ast.rs` | `src/leindex/src/ast.rs` | AST base types |
| `maestro/leindex/rust/src/ast_analyzer.rs` | `src/leindex/src/ast_analyzer.rs` | AST analyzer |
| `maestro/leindex/rust/src/callgraph.rs` | `src/leindex/src/callgraph.rs` | Call graph |
| `maestro/leindex/rust/src/cfg.rs` | `src/leindex/src/cfg.rs` | Control flow graph |
| `maestro/leindex/rust/src/dfg.rs` | `src/leindex/src/dfg.rs` | Data flow graph |
| `maestro/leindex/rust/src/five_phase.rs` | `src/leindex/src/five_phase.rs` | 5-phase analysis |
| `maestro/leindex/rust/src/multi_lang_ast.rs` | `src/leindex/src/multi_lang_ast.rs` | Multi-language AST |
| `maestro/leindex/rust/src/multi_lang_callgraph.rs` | `src/leindex/src/multi_lang_callgraph.rs` | Multi-language call graph |
| `maestro/leindex/rust/src/multi_lang_cfg.rs` | `src/leindex/src/multi_lang_cfg.rs` | Multi-language CFG |
| `maestro/leindex/rust/src/multi_lang_dfg.rs` | `src/leindex/src/multi_lang_dfg.rs` | Multi-language DFG |
| `maestro/leindex/rust/src/multi_lang_slicing.rs` | `src/leindex/src/multi_lang_slicing.rs` | Multi-language slicing |
| `maestro/leindex/rust/src/slicing.rs` | `src/leindex/src/slicing.rs` | Program slicing |

---

### API Directory (9 files)

| Current Path | New Path | Notes |
|--------------|----------|-------|
| `maestro/leindex/rust/src/api/mod.rs` | `src/leindex/src/api/mod.rs` | API module exports |
| `maestro/leindex/rust/src/api/server.rs` | `src/leindex/src/api/server.rs` | API server |
| `maestro/leindex/rust/src/api/routes.rs` | `src/leindex/src/api/routes.rs` | Route definitions |
| `maestro/leindex/rust/src/api/handlers.rs` | `src/leindex/src/api/handlers.rs` | Request handlers |
| `maestro/leindex/rust/src/api/response.rs` | `src/leindex/src/api/response.rs` | Response types |
| `maestro/leindex/rust/src/api/lattice/mod.rs` | `src/leindex/src/api/lattice/mod.rs` | Lattice API module |
| `maestro/leindex/rust/src/api/lattice/models.rs` | `src/leindex/src/api/lattice/models.rs` | Lattice models |
| `maestro/leindex/rust/src/api/lattice/handlers.rs` | `src/leindex/src/api/lattice/handlers.rs` | Lattice handlers |
| `maestro/leindex/rust/src/api/lattice/routes.rs` | `src/leindex/src/api/lattice/routes.rs` | Lattice routes |

---

### Benchmark Directory (2 files)

| Current Path | New Path | Notes |
|--------------|----------|-------|
| `maestro/leindex/rust/src/bench/simple_bench.rs` | `src/leindex/src/bench/simple_bench.rs` | Simple benchmark |
| `maestro/leindex/rust/src/bench/granular_bench.rs` | `src/leindex/src/bench/granular_bench.rs` | Granular benchmark |

---

### CLI Directory (10 files)

| Current Path | New Path | Notes |
|--------------|----------|-------|
| `maestro/leindex/rust/src/cli/mod.rs` | `src/leindex/src/cli/mod.rs` | CLI module exports |
| `maestro/leindex/rust/src/cli/analyze.rs` | `src/leindex/src/cli/analyze.rs` | Analyze command |
| `maestro/leindex/rust/src/cli/implement.rs` | `src/leindex/src/cli/implement.rs` | Implement command |
| `maestro/leindex/rust/src/cli/integrate.rs` | `src/leindex/src/cli/integrate.rs` | Integrate command |
| `maestro/leindex/rust/src/cli/leindex_cmd.rs` | `src/leindex/src/cli/leindex_cmd.rs` | LeIndex commands |
| `maestro/leindex/rust/src/cli/mcp.rs` | `src/leindex/src/cli/mcp.rs` | MCP commands |
| `maestro/leindex/rust/src/cli/memory_cmd.rs` | `src/leindex/src/cli/memory_cmd.rs` | Memory commands |
| `maestro/leindex/rust/src/cli/orchestrate.rs` | `src/leindex/src/cli/orchestrate.rs` | Orchestrate commands |
| `maestro/leindex/rust/src/cli/prompt.rs` | `src/leindex/src/cli/prompt.rs` | Prompt utilities |
| `maestro/leindex/rust/src/cli/vector_benchmark.rs` | `src/leindex/src/cli/vector_benchmark.rs` | Vector benchmark |

---

### LSP Directory (3 files)

| Current Path | New Path | Notes |
|--------------|----------|-------|
| `maestro/leindex/rust/src/lsp/mod.rs` | `src/leindex/src/lsp/mod.rs` | LSP module exports |
| `maestro/leindex/rust/src/lsp/stdio_proxy.rs` | `src/leindex/src/lsp/stdio_proxy.rs` | stdio proxy |
| `maestro/leindex/rust/src/lsp/bin/mcp_bridge.rs` | `src/leindex/src/lsp/bin/mcp_bridge.rs` | MCP bridge binary |

---

### Memory Directory (14 files)

| Current Path | New Path | Notes |
|--------------|----------|-------|
| `maestro/leindex/rust/src/memory/mod.rs` | `src/leindex/src/memory/mod.rs` | Memory module exports |
| `maestro/leindex/rust/src/memory/db.rs` | `src/leindex/src/memory/db.rs` | Database interface |
| `maestro/leindex/rust/src/memory/lsp_manager.rs` | `src/leindex/src/memory/lsp_manager.rs` | LSP manager |
| `maestro/leindex/rust/src/memory/lsp_pool.rs` | `src/leindex/src/memory/lsp_pool.rs` | LSP pool |
| `maestro/leindex/rust/src/memory/mcp_discovery.rs` | `src/leindex/src/memory/mcp_discovery.rs` | MCP discovery |
| `maestro/leindex/rust/src/memory/mcp_pool.rs` | `src/leindex/src/memory/mcp_pool.rs` | MCP pool |
| `maestro/leindex/rust/src/memory/migration.rs` | `src/leindex/src/memory/migration.rs` | Migration support |
| `maestro/leindex/rust/src/memory/models.rs` | `src/leindex/src/memory/models.rs` | Data models |
| `maestro/leindex/rust/src/memory/schema.rs` | `src/leindex/src/memory/schema.rs` | Database schema |
| `maestro/leindex/rust/src/memory/scanner.rs` | `src/leindex/src/memory/scanner.rs` | Code scanner |
| `maestro/leindex/rust/src/memory/search.rs` | `src/leindex/src/memory/search.rs` | Search interface |
| `maestro/leindex/rust/src/memory/service.rs` | `src/leindex/src/memory/service.rs` | Memory service |
| `maestro/leindex/rust/src/memory/session_manager.rs` | `src/leindex/src/memory/session_manager.rs` | Session management |
| `maestro/leindex/rust/src/memory/turso_backend.rs` | `src/leindex/src/memory/turso_backend.rs` | Turso backend |

---

### Migrations Directory (1 file)

| Current Path | New Path | Notes |
|--------------|----------|-------|
| `maestro/leindex/rust/src/migrations/mod.rs` | `src/leindex/src/migrations/mod.rs` | Migrations module |

---

### Multiplexer Directory (3 files)

| Current Path | New Path | Notes |
|--------------|----------|-------|
| `maestro/leindex/rust/src/multiplexer/mod.rs` | `src/leindex/src/multiplexer/mod.rs` | Multiplexer module |
| `maestro/leindex/rust/src/multiplexer/tmux.rs` | `src/leindex/src/multiplexer/tmux.rs` | Tmux implementation (to be deprecated) |
| `maestro/leindex/rust/src/multiplexer/zellij.rs` | `src/leindex/src/multiplexer/zellij.rs` | Zellij fallback |

---

### Orchestrate Directory (13 files)

| Current Path | New Path | Notes |
|--------------|----------|-------|
| `maestro/leindex/rust/src/orchestrate/mod.rs` | `src/leindex/src/orchestrate/mod.rs` | Orchestrate module |
| `maestro/leindex/rust/src/orchestrate/context.rs` | `src/leindex/src/orchestrate/context.rs` | Context handling |
| `maestro/leindex/rust/src/orchestrate/control.rs` | `src/leindex/src/orchestrate/control.rs` | Control flow |
| `maestro/leindex/rust/src/orchestrate/diagnostics.rs` | `src/leindex/src/orchestrate/diagnostics.rs` | Diagnostics |
| `maestro/leindex/rust/src/orchestrate/engine.rs` | `src/leindex/src/orchestrate/engine.rs` | Conductor engine |
| `maestro/leindex/rust/src/orchestrate/lsp_client.rs` | `src/leindex/src/orchestrate/lsp_client.rs` | LSP client |
| `maestro/leindex/rust/src/orchestrate/model.rs` | `src/leindex/src/orchestrate/model.rs` | Data models |
| `maestro/leindex/rust/src/orchestrate/parser.rs` | `src/leindex/src/orchestrate/parser.rs` | Track parser |
| `maestro/leindex/rust/src/orchestrate/prompts.rs` | `src/leindex/src/orchestrate/prompts.rs` | Prompt templates |
| `maestro/leindex/rust/src/orchestrate/rate_limit.rs` | `src/leindex/src/orchestrate/rate_limit.rs` | Rate limiting |
| `maestro/leindex/rust/src/orchestrate/rate_limit_detector.rs` | `src/leindex/src/orchestrate/rate_limit_detector.rs` | Rate limit detection |
| `maestro/leindex/rust/src/orchestrate/runner.rs` | `src/leindex/src/orchestrate/runner.rs` | Agent runner |
| `maestro/leindex/rust/src/orchestrate/setup.rs` | `src/leindex/src/orchestrate/setup.rs` | Orchestrate setup |
| `maestro/leindex/rust/src/orchestrate/state.rs` | `src/leindex/src/orchestrate/state.rs` | State management |

---

### Setup Directory (4 files)

| Current Path | New Path | Notes |
|--------------|----------|-------|
| `maestro/leindex/rust/src/setup/mod.rs` | `src/leindex/src/setup/mod.rs` | Setup module |
| `maestro/leindex/rust/src/setup/distro.rs` | `src/leindex/src/setup/distro.rs` | Distribution detection |
| `maestro/leindex/rust/src/setup/package_manager.rs` | `src/leindex/src/setup/package_manager.rs` | Package manager abstraction |
| `maestro/leindex/rust/src/setup/password.rs` | `src/leindex/src/setup/password.rs` | Password management |

---

### Vector Directory (13 files)

| Current Path | New Path | Notes |
|--------------|----------|-------|
| `maestro/leindex/rust/src/vector/mod.rs` | `src/leindex/src/vector/mod.rs` | Vector module |
| `maestro/leindex/rust/src/vector/adaptive.rs` | `src/leindex/src/vector/adaptive.rs` | Adaptive router |
| `maestro/leindex/rust/src/vector/benchmark_tests.rs` | `src/leindex/src/vector/benchmark_tests.rs` | Benchmark tests |
| `maestro/leindex/rust/src/vector/cache.rs` | `src/leindex/src/vector/cache.rs` | Vector cache |
| `maestro/leindex/rust/src/vector/concurrency_tests.rs` | `src/leindex/src/vector/concurrency_tests.rs` | Concurrency tests |
| `maestro/leindex/rust/src/vector/diagnostics.rs` | `src/leindex/src/vector/diagnostics.rs` | Diagnostics |
| `maestro/leindex/rust/src/vector/hnsw_store.rs` | `src/leindex/src/vector/hnsw_store.rs` | HNSW store |
| `maestro/leindex/rust/src/vector/metadata.rs` | `src/leindex/src/vector/metadata.rs` | Metadata |
| `maestro/leindex/rust/src/vector/migrations.rs` | `src/leindex/src/vector/migrations.rs` | Vector migrations |
| `maestro/leindex/rust/src/vector/report.rs` | `src/leindex/src/vector/report.rs` | Reports |
| `maestro/leindex/rust/src/vector/simd.rs` | `src/leindex/src/vector/simd.rs` | SIMD operations |
| `maestro/leindex/rust/src/vector/store.rs` | `src/leindex/src/vector/store.rs` | Store trait |
| `maestro/leindex/rust/src/vector/turso_store.rs` | `src/leindex/src/vector/turso_store.rs` | Turso store |

---

## Other Files to Consider

### Binary Files
| Current Path | New Path | Notes |
|--------------|----------|-------|
| `maestro/leindex/rust/bin/setup_main.rs` | `src/leindex/bin/setup_main.rs` | Setup binary |

### Build Configuration
| Current Path | New Path | Notes |
|--------------|----------|-------|
| `maestro/leindex/rust/Cargo.toml` | `src/leindex/Cargo.toml` | Package manifest |

### Generated Cache (Ignore)
| Path | Notes |
|------|-------|
| `maestro/leindex/rust/src/setup/.leindex/` | LeIndex analysis cache - DO NOT COPY |

---

## Import Path Updates Required

### Internal Imports (within leindex-core)

All internal imports using `crate::` will continue to work without changes.

### External Imports (from other crates)

Files that import from `leindex_core` will need updates:

1. **Cockpit** (`crates/cockpit/src/*.rs`)
   - `use leindex_core::multiplexer::TmuxMultiplexer`
   - `use leindex_core::memory::*`
   - `use leindex_core::orchestrate::*`

2. **CLI** (`crates/cli/src/main.rs`)
   - `use leindex_core::*`

3. **Pi-Mono** (`crates/pi-mono/src/*.rs`)
   - Any `leindex_core` imports

4. **LSP-Bridge** (`crates/lsp-bridge/src/*.rs`)
   - Any `leindex_core` imports

**Note:** The `leindex_core` crate name remains the same, only the file location changes. External imports should continue to work after workspace update.

---

## Summary Statistics

| Category | Count |
|----------|-------|
| Root files | 17 |
| API files | 9 |
| Benchmark files | 2 |
| CLI files | 10 |
| LSP files | 3 |
| Memory files | 14 |
| Migrations files | 1 |
| Multiplexer files | 3 |
| Orchestrate files | 13 |
| Setup files | 4 |
| Vector files | 13 |
| **Total** | **93** |

---

## Execution Plan

1. Create `src/` directory at repository root
2. Create `src/leindex/` directory
3. Copy all files maintaining directory structure
4. Update `src/leindex/Cargo.toml` (if needed)
5. Update root `Cargo.toml` workspace members
6. Update import paths in dependent crates
7. Remove old `maestro/leindex/rust/` directory after verification
8. Run full test suite to verify
