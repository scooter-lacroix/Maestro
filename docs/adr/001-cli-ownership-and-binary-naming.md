# ADR 001: CLI Ownership and Binary Naming

## Status
**Accepted** | 2026-01-22

## Context

Maestro is a Rust-first framework. All core functionality (CLI, TUI, MCP, analysis) is implemented in Rust. The `cli.py` file at `maestro/cli.py` is legacy code that should be removed or archived.

Current state:
- Rust binary: `maestro` (from `leindex-analyzers` package)
- Legacy Python file: `maestro/cli.py` (should be archived)
- Go TUI: already archived

## Decision

### Primary Entry Point: Rust Binary

The `maestro` binary is the **sole** user-facing CLI. It is built from Rust source using Cargo.

### Binary Naming Strategy

| Binary | Name | Purpose | Entry Point |
|--------|------|---------|-------------|
| Main CLI | `maestro` | Primary Rust CLI with all commands | `crates/cli/src/main.rs` |
| Setup Wizard | `maestro-setup` | Installation TUI | `leindex-core/bin/setup_main.rs` |
| LSP Bridge | `maestro-lsp-mcp-bridge` | LSP to MCP protocol bridge | `crates/lsp-bridge/src/main.rs` |
| Benchmarks | `leindex-benchmark-*` | Vector store benchmarks | `leindex-core/bin/*` |

### Archive Legacy Python Code

The `maestro/cli.py` file must be:
1. Moved to `maestro/archive/legacy-python-cli/`
2. Removed from any runtime references
3. Documented as historical reference only

### User Experience

```bash
# Primary command (Rust)
$ maestro --help
# All functionality: tui, analyze, implement, memory, mcp, etc.

# TUI
$ maestro tui
# Launches Rust Cockpit TUI

# Analysis
$ maestro analyze src/lib.rs
# Runs LeIndex analysis

# Setup wizard
$ maestro-setup
# Installation/configuration wizard
```

### Crate Structure

```
crates/
├── cli/              # maestro-cli (produces maestro binary)
├── cockpit/          # maestro-cockpit (Ratatui TUI library)
└── lsp-bridge/       # maestro-lsp-mcp-bridge (produces bridge binary)

leindex/              # leindex-core (core library + setup binary)
└── rust/
```

### Dependency Rules (One-Way)

```
cli → cockpit + core
cockpit → core
core ↛ cockpit (forbidden)
```

## Rationale

1. **Rust-first architecture**: All core functionality is in Rust for performance and safety
2. **Single binary distribution**: One `maestro` command provides all functionality
3. **Clear separation**: Each crate has single responsibility
4. **Future-proof**: Easy to add orchestrate crate later

## Consequences

### Positive
- Single `maestro` command - no confusion
- High performance (native Rust)
- Memory safety guarantees
- Clear crate boundaries enforced by Cargo

### Negative
- Users building from source need Rust toolchain (mitigated by pre-built binaries)

### Neutral
- Legacy Python CLI archived for historical reference
- Makefile needs updates (removing Go/Python TUI references)

## Implementation

See `docs/adr/002-crate-reorganization.md` for detailed migration plan.

## Actions Required

1. Archive `maestro/cli.py` to `maestro/archive/legacy-python-cli/`
2. Remove any references to Python CLI in documentation
3. Update install scripts to use Rust binary only
4. Ensure `maestro` binary is installed to `~/.local/bin/`

## Alternatives Considered

1. **Keep Python CLI as delegator**
   - Rejected: Maestro is Rust-first; Python adds unnecessary complexity
   - Rejected: All functionality already in Rust

2. **Use different binary name for Rust**
   - Rejected: Creates confusion; `maestro` is the established name
   - Rejected: No reason to rename established binary

## Related Decisions
- ADR 002: Crate Reorganization
- Spec: `maestro/tracks/v2-5_20260121/spec.md`
