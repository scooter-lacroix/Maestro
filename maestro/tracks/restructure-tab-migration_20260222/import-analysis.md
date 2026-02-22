# Import Path Analysis: LeIndex Restructure

**Track:** restructure-tab-migration_20260222
**Phase:** 1 - Project Restructuring
**Task:** 1.1.2 - Import Path Analysis
**Created:** 2026-02-22

---

## Overview

This document analyzes all imports of `leindex_core` across the codebase to determine what changes are required after restructuring.

**Key Finding:** The `leindex_core` crate name remains unchanged. External imports should continue to work after workspace configuration updates. No code changes are required in most cases.

---

## Summary

| Category | Files | Changes Required |
|----------|-------|------------------|
| Internal (within leindex_core) | 7 | None (use `crate::` or relative paths) |
| External (crates/) | 27 | None (crate name unchanged) |
| **Total** | **34** | **0** |

---

## Files Using `leindex_core::` Imports

### Cockpit Crate (24 files)

| File | Import Lines | Required Changes |
|------|--------------|------------------|
| `crates/cockpit/src/lib.rs` | `use leindex_core::*` | None |
| `crates/cockpit/src/app.rs` | Multiplexer, orchestrate imports | None |
| `crates/cockpit/src/orchestrate.rs` | `use leindex_core::orchestrate::*` | None |
| `crates/cockpit/src/maestro_paths.rs` | Multiplexer imports | None |
| `crates/cockpit/src/state/types.rs` | Model imports | None |
| `crates/cockpit/src/conductor/model.rs` | `leindex_core::orchestrate::model` | None |
| `crates/cockpit/src/conductor/pane.rs` | Multiplexer, orchestrate | None |
| `crates/cockpit/src/conductor/observer.rs` | Multiplexer imports | None |
| `crates/cockpit/src/conductor/tests.rs` | Test imports | None |
| `crates/cockpit/src/conductor/details_panel.rs` | Orchestrate imports | None |
| `crates/cockpit/src/conductor/track_tree.rs` | Model imports | None |
| `crates/cockpit/src/conductor/conflict_panel.rs` | Model imports | None |
| `crates/cockpit/src/conductor/keybindings.rs` | Orchestrate imports | None |
| `crates/cockpit/src/conductor/polling.rs` | Memory imports | None |
| `crates/cockpit/src/conductor/memory_browser.rs` | Memory imports | None |
| `crates/cockpit/src/conductor/parallel_view.rs` | Model imports | None |
| `crates/cockpit/src/conductor/iteration_history.rs` | Model imports | None |
| `crates/cockpit/src/tabs/dashboard.rs` | Memory status imports | None |
| `crates/cockpit/src/tabs/sessions.rs` | Memory status imports | None |
| `crates/cockpit/src/tabs/projects.rs` | Memory imports | None |
| `crates/cockpit/src/tabs/lsps.rs` | LSP status imports | None |
| `crates/cockpit/src/tabs/lsp_registry.rs` | LSP imports | None |
| `crates/cockpit/src/modals/sessions.rs` | Memory imports | None |

### CLI Crate (2 files)

| File | Import Lines | Required Changes |
|------|--------------|------------------|
| `crates/cli/src/main.rs` | `use leindex_core::*` | None |
| `crates/cli/src/commands/implement.rs` | CLI imports | None |

### LSP-Bridge Crate (2 files)

| File | Import Lines | Required Changes |
|------|--------------|------------------|
| `crates/lsp-bridge/src/main.rs` | `use leindex_core::lsp::*` | None |
| `crates/lsp-bridge/src/mcp_bridge.rs` | LSP imports | None |

---

## Internal Imports (leindex_core itself)

These files are part of leindex-core and use internal imports. They will continue to work without changes:

| File | Import Type | Required Changes |
|------|-------------|------------------|
| `maestro/leindex/rust/bin/setup_main.rs` | `use leindex_core::*` | None |
| `maestro/leindex/rust/src/bench/simple_bench.rs` | `use leindex_core::*` | None |
| `maestro/leindex/rust/src/bench/granular_bench.rs` | `use leindex_core::*` | None |
| `maestro/leindex/rust/src/cli/vector_benchmark.rs` | `use leindex_core::*` | None |
| `maestro/leindex/rust/src/main.rs` | `use leindex_core::*` | None |
| `maestro/leindex/rust/src/memory/mod.rs` | `crate::` re-exports | None |
| `maestro/leindex/rust/src/lsp/mod.rs` | `crate::` re-exports | None |

---

## Import Types

### Type 1: Direct leindex_core imports

```rust
use leindex_core::multiplexer::TmuxMultiplexer;
use leindex_core::memory::service::MemoryService;
use leindex_core::orchestrate::engine::OrchestrateEngine;
```

**Status:** No changes required. The crate name remains `leindex_core`.

### Type 2: Wildcard imports

```rust
use leindex_core::*;
```

**Status:** No changes required.

### Type 3: Type aliases

```rust
use leindex_core::orchestrate::model::{Track, Task};
```

**Status:** No changes required.

### Type 4: Internal crate imports

```rust
// Within leindex_core
use crate::memory::service::MemoryService;
```

**Status:** No changes required. Internal `crate::` paths are relative to the crate root.

---

## Workspace Configuration Updates Required

While no code changes are required, the workspace configuration must be updated:

### Root Cargo.toml

**Before:**
```toml
[workspace]
members = [
    "crates/...",
    "maestro/leindex/rust",
]
```

**After:**
```toml
[workspace]
members = [
    "crates/...",
    "src/leindex",
]
```

### Dependency Declarations

No changes required in dependent crates' `Cargo.toml` files:

```toml
# crates/cockpit/Cargo.toml
[dependencies]
leindex-core = { path = "maestro/leindex/rust" }  # BEFORE
leindex-core = { path = "src/leindex" }           # AFTER
```

---

## Module Paths Within leindex_core

### Internal Module Structure (unchanged)

```
leindex_core/
├── lib.rs (crate root)
├── api/
├── bench/
├── cli/
├── lsp/
├── memory/
├── migrations/
├── multiplexer/
├── orchestrate/
├── setup/
└── vector/
```

**All internal module paths remain the same.** Public API exports are unchanged.

### Public API Exports (from lib.rs)

The following exports are re-exported and will continue to work:

```rust
// Memory
pub use memory::*;
pub use memory::models::*;
pub use memory::service::*;
pub use memory::turso_backend::*;

// Orchestrate
pub use orchestrate::*;

// Multiplexer
pub use multiplexer::*;

// CLI
pub use cli::*;

// etc.
```

---

## Verification Steps

After restructuring, verify:

1. **Build succeeds:**
   ```bash
   cargo build --workspace
   ```

2. **Tests pass:**
   ```bash
   cargo test --workspace
   ```

3. **No broken imports:**
   ```bash
   cargo check --workspace
   ```

4. **Specific import checks:**
   - Cockpit can import `leindex_core::multiplexer::TmuxMultiplexer`
   - CLI can import `leindex_core::*`
   - LSP-bridge can import `leindex_core::lsp::*`

---

## Potential Issues and Mitigations

### Issue 1: Absolute path references in documentation

**Problem:** Documentation may reference `maestro/leindex/rust/src/` paths.

**Solution:** Update all documentation files (README.md, CLAUDE.md, ADRs).

### Issue 2: Build scripts or procedural macros

**Problem:** build.rs files might have path dependencies.

**Solution:** Check and update any build.rs files in leindex-core.

### Issue 3: Test data file paths

**Problem:** Tests may have hardcoded paths to test data.

**Solution:** Update test fixtures if they reference source location.

---

## Conclusion

**No code changes are required** for the restructuring. The `leindex_core` crate name remains unchanged, and all imports will continue to work after updating the workspace configuration.

The only changes required are:
1. Update root `Cargo.toml` workspace members
2. Update dependent crates' `Cargo.toml` `leindex-core` path dependency
3. Update documentation references to source paths
