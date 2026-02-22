# Workspace Dependency Graph: LeIndex Restructure

**Track:** restructure-tab-migration_20260222
**Phase:** 1 - Project Restructuring
**Task:** 1.1.3 - Workspace Dependency Graph
**Created:** 2026-02-22

---

## Overview

This document describes the current workspace dependency structure and changes required after restructuring.

---

## Current Workspace Structure

```
Cargo.toml (workspace root)
│
├── maestro/leindex/rust/      [leindex-core]
│   └── (93 source files)
│
├── crates/
│   ├── cockpit/               [maestro-cockpit]
│   ├── cli/                   [maestro-cli]
│   ├── lsp-bridge/            [maestro-lsp-bridge]
│   ├── pi-mono/               [maestro-pi-mono]
│   ├── ktop_collectors/       [ktop_collectors]
│   ├── core/                  [maestro-core]
│   └── gateway/               [maestro-gateway]
│
└── vendor/libsql/             (vendored dependency)
```

---

## Dependency Graph

### Visual Representation

```
                    ┌─────────────────────────────────────┐
                    │          leindex-core               │
                    │    (maestro/leindex/rust → src/leindex)    │
                    │  - memory service                   │
                    │  - orchestrate engine               │
                    │  - multiplexer                      │
                    │  - vector search                    │
                    │  - CLI commands                     │
                    └─────────────────────────────────────┘
                                     │
                    ┌────────────────┼────────────────┐
                    │                │                │
                    ▼                ▼                ▼
        ┌──────────────────┐  ┌──────────┐  ┌──────────────┐
        │   maestro-cli    │  │ cockpit  │  │ lsp-bridge   │
        │                  │  │          │  │              │
        │ depends on:      │  │ depends: │  │ depends on:  │
        │ - leindex-core   │  │ - leindex│  │ - leindex    │
        │ - cockpit        │  │ - core   │  │              │
        │ - pi-mono        │  │ - pi-mono│  │              │
        └──────────────────┘  │ - ktop   │  └──────────────┘
                              └──────────┘
                                     │
                    ┌────────────────┼────────────────┐
                    │                │                │
                    ▼                ▼                ▼
        ┌──────────────────┐  ┌──────────┐  ┌──────────────┐
        │   maestro-pi-mono│  │maestro-  │  │ ktop_        │
        │                  │  │ core     │  │ collectors   │
        │ depends on:      │  │          │  │              │
        │ (none - standalone│  │ depends: │  │ depends on:  │
        │  workspace lib)  │  │ - none   │  │ - none       │
        └──────────────────┘  └──────────┘  └──────────────┘
```

---

## Detailed Crate Dependencies

### leindex-core (maestro/leindex/rust → src/leindex)

**Crate Name:** `leindex-core`
**Package Name:** `leindex-core`
**Type:** Library (rlib)
**Binaries:**
- `maestro-setup` (from bin/setup_main.rs)
- `leindex-benchmark-report`
- `leindex-simple-bench`
- `leindex-granular-bench`

**External Dependencies:**
- tokio, axum, tree-sitter, rusqlite/libsql, tantivy
- (see maestro/leindex/rust/Cargo.toml for full list)

**Dependents (crates that depend on leindex-core):**
- maestro-cockpit
- maestro-cli
- lsp-bridge

**Required Changes After Restructure:**
- Path in workspace Cargo.toml: `maestro/leindex/rust` → `src/leindex`
- Path in dependent crates: `../../maestro/leindex/rust` → `../../src/leindex`

---

### maestro-cockpit

**Crate Name:** `maestro-cockpit`
**Package Name:** `maestro-cockpit`
**Type:** Library (rlib) + Binary

**Dependencies:**
```toml
[dependencies]
leindex-core = { path = "../../maestro/leindex/rust" }  # ← UPDATE THIS
maestro-core = { path = "../core" }
maestro-pi-mono = { path = "../pi-mono" }
ktop_collectors = { path = "../ktop_collectors" }
```

**Required Changes:**
```toml
leindex-core = { path = "../../src/leindex" }  # NEW PATH
```

---

### maestro-cli

**Crate Name:** `maestro-cli`
**Package Name:** `maestro-cli`
**Type:** Binary

**Dependencies:**
```toml
[dependencies]
leindex-core = { path = "../../maestro/leindex/rust" }  # ← UPDATE THIS
maestro-cockpit = { path = "../cockpit" }
maestro-pi-mono = { path = "../pi-mono" }
```

**Required Changes:**
```toml
leindex-core = { path = "../../src/leindex" }  # NEW PATH
```

---

### lsp-bridge

**Crate Name:** (check package name)
**Type:** Binary

**Dependencies:**
```toml
[dependencies]
leindex-core = { path = "../../maestro/leindex/rust" }  # ← UPDATE THIS
```

**Required Changes:**
```toml
leindex-core = { path = "../../src/leindex" }  # NEW PATH
```

---

### maestro-pi-mono

**Crate Name:** `maestro_pi_mono`
**Package Name:** `maestro-pi-mono`
**Type:** Library (rlib)

**Dependencies:**
- None from workspace (standalone)

**Required Changes:**
- None

---

### maestro-core

**Crate Name:** `maestro_core`
**Package Name:** `maestro-core`
**Type:** Library (rlib)

**Dependencies:**
- (check lib.rs for details)

**Required Changes:**
- None (doesn't depend on leindex-core)

---

### ktop_collectors

**Crate Name:** `ktop_collectors`
**Package Name:** `ktop_collectors`
**Type:** Library (rlib)

**Dependencies:**
- System metrics collection only

**Required Changes:**
- None

---

### maestro-gateway

**Crate Name:** (check package name)
**Type:** (check)

**Dependencies:**
- (pending verification)

**Required Changes:**
- (pending verification)

---

## Architectural Rules (Dependency Policy)

### STRICT ONE-WAY DEPENDENCY RULE

```
cli → cockpit + leindex-core + pi-mono
cockpit → leindex-core + maestro-core + pi-mono
pi-mono → (standalone)
leindex-core ↛ cockpit (FORBIDDEN)
```

**Policy Enforcement:** `make policy-check`

The restructuring must NOT violate this rule. Since we're only changing file locations (not crate names or dependencies), the rule remains intact.

---

## Workspace Configuration Changes

### Root Cargo.toml

**Current:**
```toml
[workspace]
members = [
    "maestro/leindex/rust",     # ← REMOVE THIS
    "crates/cockpit",
    "crates/cli",
    "crates/lsp-bridge",
    "crates/pi-mono",
    "crates/ktop_collectors",
    "crates/core",
    "crates/gateway",
]
```

**After Restructure:**
```toml
[workspace]
members = [
    "src/leindex",              # ← ADD THIS
    "crates/cockpit",
    "crates/cli",
    "crates/lsp-bridge",
    "crates/pi-mono",
    "crates/ktop_collectors",
    "crates/core",
    "crates/gateway",
]
```

---

## Path Updates Summary

| File | Current Path | New Path |
|------|--------------|----------|
| `Cargo.toml` (root) | `maestro/leindex/rust` | `src/leindex` |
| `crates/cockpit/Cargo.toml` | `../../maestro/leindex/rust` | `../../src/leindex` |
| `crates/cli/Cargo.toml` | `../../maestro/leindex/rust` | `../../src/leindex` |
| `crates/lsp-bridge/Cargo.toml` | `../../maestro/leindex/rust` | `../../src/leindex` |

---

## Build Order

The build order remains unchanged:

1. **leindex-core** (no internal workspace dependencies)
2. **pi-mono** (standalone)
3. **maestro-core** (if no leindex-core dependency)
4. **ktop_collectors** (standalone)
5. **cockpit** (depends on leindex-core)
6. **lsp-bridge** (depends on leindex-core)
7. **cli** (depends on leindex-core, cockpit, pi-mono)
8. **gateway** (if applicable)

---

## Verification Commands

After restructuring, verify the dependency graph:

```bash
# Check workspace builds
cargo build --workspace

# Verify dependency tree
cargo tree -p maestro-cockpit
cargo tree -p maestro-cli
cargo tree -p lsp-bridge

# Check for forbidden dependencies
make policy-check

# Verify no circular dependencies
cargo tree --duplicates
```

---

## Risk Assessment

### Low Risk Changes
- File location changes only
- Crate names unchanged
- Public API unchanged
- Internal imports use `crate::` (location-agnostic)

### Medium Risk Areas
- Documentation with hardcoded paths
- Build scripts with path dependencies
- Test fixtures with path dependencies

### Mitigation
- Comprehensive test suite (193+ tests)
- Policy check enforcement
- Documentation updates
