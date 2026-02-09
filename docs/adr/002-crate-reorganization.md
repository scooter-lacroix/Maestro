# ADR 002: Crate Reorganization for Maestro v2.5

## Status
**Proposed** | 2026-01-22

## Context

The current Rust codebase has structural issues:
- Package named `leindex-analyzers` but produces `maestro` binary (confusing)
- 6000+ line monolithic `tui.rs` file (maintenance nightmare)
- UI code mixed with core analysis logic (tight coupling)
- No clear boundary between CLI, TUI, and core libraries
- Legacy `cli.py` should be archived

## Decision

### Target Crate Structure

```
maestro/
├── crates/
│   ├── cli/              # maestro-cli (~200 LOC)
│   │   ├── src/
│   │   │   ├── main.rs   # Binary entry point (produces "maestro")
│   │   │   └── commands/
│   │   │       ├── mod.rs
│   │   │       ├── tui.rs      # Delegates to cockpit
│   │   │       ├── analyze.rs  # Delegates to core
│   │   │       ├── implement.rs
│   │   │       ├── memory.rs
│   │   │       └── mcp.rs
│   │   └── Cargo.toml
│   │
│   ├── cockpit/          # maestro-cockpit (library, ~3000 LOC)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── app/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── state.rs
│   │   │   │   └── render.rs
│   │   │   ├── ui/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── tabs/
│   │   │   │   └── widgets/
│   │   │   ├── actions/
│   │   │   │   ├── mod.rs
│   │   │   │   └── handlers.rs
│   │   │   └── theme.rs
│   │   └── Cargo.toml
│   │
│   └── lsp-bridge/       # maestro-lsp-mcp-bridge (binary)
│       ├── src/
│       │   └── main.rs
│       └── Cargo.toml
│
├── archive/
│   └── legacy-python-cli/  # Archived cli.py
│       └── cli.py
│
└── leindex/
    └── rust/            # leindex-core (library + setup binary)
        ├── Cargo.toml   # Workspace root
        ├── src/
        │   ├── lib.rs
        │   ├── analyzers/
        │   │   ├── mod.rs
        │   │   ├── ast.rs
        │   │   ├── callgraph.rs
        │   │   ├── cfg.rs
        │   │   ├── dfg.rs
        │   │   └── slicing.rs
        │   ├── analysis/
        │   │   ├── mod.rs
        │   │   └── five_phase.rs
        │   ├── memory/
        │   │   ├── mod.rs
        │   │   ├── service.rs
        │   │   ├── models.rs
        │   │   ├── mcp_pool.rs
        │   │   ├── session_manager.rs
        │   │   ├── lsp_manager.rs
        │   │   └── turso_backend.rs
        │   ├── vector/
        │   │   ├── mod.rs
        │   │   ├── store.rs
        │   │   ├── hnsw_store.rs
        │   │   ├── turso_store.rs
        │   │   └── adaptive.rs
        │   ├── lsp/
        │   │   ├── mod.rs
        │   │   ├── stdio_proxy.rs
        │   │   └── mcp_bridge.rs
        │   ├── multiplexer/
        │   │   ├── mod.rs
        │   │   └── tmux.rs
        │   ├── language.rs
        │   ├── token_format.rs
        │   └── config.rs
        └── bin/
            └── setup_main.rs
```

### Workspace Cargo.toml

```toml
[workspace]
members = [
    "leindex/rust",
    "crates/cli",
    "crates/cockpit",
    "crates/lsp-bridge",
]
resolver = "2"

[workspace.package]
version = "2.5.0"
edition = "2021"
authors = ["Maestro Project"]

[workspace.dependencies]
# Shared dependencies for all crates
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
```

### Crate Dependencies

```toml
# crates/cli/Cargo.toml
[package]
name = "maestro-cli"

[dependencies]
leindex-core = { path = "../../leindex/rust" }
maestro-cockpit = { path = "../cockpit" }
clap = { version = "4.5", features = ["derive"] }

[[bin]]
name = "maestro"
path = "src/main.rs"

# crates/cockpit/Cargo.toml
[package]
name = "maestro-cockpit"

[dependencies]
leindex-core = { path = "../../leindex/rust" }
ratatui = "0.29"
crossterm = "0.28"

# crates/lsp-bridge/Cargo.toml
[package]
name = "maestro-lsp-mcp-bridge"

[dependencies]
leindex-core = { path = "../../leindex/rust" }

[[bin]]
name = "maestro-lsp-mcp-bridge"
path = "src/main.rs"

# leindex/rust/Cargo.toml
[package]
name = "leindex-core"

[dependencies]
# All analysis/memory/vector dependencies

[[bin]]
name = "maestro-setup"
path = "bin/setup_main.rs"
```

## Module Migration Map

| Current Location | Target Crate | New Module Path |
|------------------|--------------|-----------------|
| `src/cli/tui.rs` | `maestro-cockpit` | `src/app/mod.rs` |
| `src/cli/theme.rs` | `maestro-cockpit` | `src/theme.rs` |
| `src/cli/analyze.rs` | `maestro-cli` | `src/commands/analyze.rs` |
| `src/cli/implement.rs` | `maestro-cli` | `src/commands/implement.rs` |
| `src/cli/memory.rs` | `maestro-cli` | `src/commands/memory.rs` |
| `src/cli/mcp.rs` | `maestro-cli` | `src/commands/mcp.rs` |
| `src/lsp/bin/mcp_bridge.rs` | `maestro-lsp-mcp-bridge` | `src/main.rs` |
| `src/main.rs` | `maestro-cli` | `src/main.rs` |
| `src/five_phase.rs` | `leindex-core` | `src/analysis/five_phase.rs` |
| All `src/memory/*` | `leindex-core` | `src/memory/*` (unchanged) |
| All `src/vector/*` | `leindex-core` | `src/vector/*` (unchanged) |
| All `src/lsp/*` (except bin) | `leindex-core` | `src/lsp/*` (unchanged) |
| `maestro/cli.py` | archive | `archive/legacy-python-cli/` |

## Implementation Phases

### Phase 0: Archive Legacy Python
1. Create `maestro/archive/legacy-python-cli/`
2. Move `maestro/cli.py` to archive
3. Add README explaining it's historical reference only

### Phase 1: Workspace Setup
1. Create `crates/` directory structure
2. Add `[workspace]` to `leindex/rust/Cargo.toml`
3. Create placeholder `Cargo.toml` files
4. Verify `cargo build --workspace` succeeds

### Phase 2: Extract leindex-core
1. Rename package: `leindex-analyzers` → `leindex-core`
2. Move `src/five_phase.rs` → `src/analysis/`
3. Update all imports across codebase
4. Run tests: `cargo test -p leindex-core`

### Phase 3: Extract maestro-cockpit
1. Create `crates/cockpit/` structure
2. Move `src/cli/tui.rs` → `crates/cockpit/src/app/mod.rs`
3. Move `src/cli/theme.rs` → `crates/cockpit/src/theme.rs`
4. Split monolithic file into modules:
   - `app/state.rs` - State structs
   - `app/render.rs` - Render functions
   - `ui/tabs/` - Tab-specific UI
   - `actions/` - Event handlers
5. Update imports to use `leindex_core`
6. Run tests: `cargo test -p maestro-cockpit`

### Phase 4: Extract maestro-cli
1. Create `crates/cli/src/commands/`
2. Move command implementations from `src/cli/`
3. Update `main.rs` to delegate to cockpit for TUI
4. Remove `[[bin]] maestro` from leindex-core
5. Run tests: `cargo test -p maestro-cli`

### Phase 5: Extract lsp-bridge
1. Create `crates/lsp-bridge/`
2. Move `src/lsp/bin/mcp_bridge.rs` → `crates/lsp-bridge/src/main.rs`
3. Remove `[[bin]]` from leindex-core
4. Run tests: `cargo test -p maestro-lsp-mcp-bridge`

### Phase 6: Integration Updates
1. Update Makefile (remove Go TUI, add Rust builds)
2. Update documentation (all Rust, no Python)
3. Update installer scripts
4. Full test: `cargo test --workspace`

### Phase 7: Verification
1. Test all binaries
2. Check for circular deps: `cargo tree`
3. Performance benchmarks
4. Documentation updates

## Rationale

1. **Separation of Concerns**: Each crate has single responsibility
2. **Maintainability**: 6000 LOC file split into focused modules
3. **Testability**: Clear boundaries enable focused testing
4. **Reusability**: Cockpit can be used as library by other tools
5. **Future-Proof**: Easy to add orchestrate crate later
6. **Rust-first**: All functionality in native Rust

## Consequences

### Positive
- Clear crate boundaries enforced by Cargo
- Easier onboarding (new devs can understand structure)
- Better test coverage (focused unit tests per crate)
- Potential for external cockpit consumers
- Rust-first architecture (performance + safety)

### Negative
- Large migration effort (7 phases)
- Temporary breakage risk during migration
- Learning curve for contributors

### Mitigations
- Incremental phases with testing at each step
- Feature branch for isolation
- Rollback plan per phase
- Comprehensive documentation

## Related Decisions
- ADR 001: CLI Ownership and Binary Naming (Rust-only)
- Spec: `maestro/tracks/v2-5_20260121/spec.md`
