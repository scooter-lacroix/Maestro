# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Maestro v2.5 is a unified development framework for AI-assisted software engineering with a **Rust-first architecture**. It provides spec-driven development, track-based task management, code analysis (LeIndex/TLDR), and a Terminal UI (Cockpit).

## Build, Test, and Lint Commands

```bash
# Build entire workspace (release mode)
make build
# Or directly: cargo build --workspace --release

# Development build (faster, no optimizations)
make dev-build
# Or: cargo build --workspace

# Run all Rust tests
make test
# Or: cargo test --workspace

# Run tests for a specific crate
cargo test -p maestro-cockpit
cargo test -p leindex-core
cargo test -p maestro-cli

# Run a single test
cargo test -p leindex-core test_name

# Check code without full build (fast feedback)
make check
# Or: cargo check --workspace

# Run clippy linter
make lint
# Or: cargo clippy --workspace --all-targets

# Format code
make fmt
# Or: cargo fmt --all

# Clean build artifacts
make clean

# Install binaries to ~/.cargo/bin
make install-local

# Policy check (enforce architectural rules)
make policy-check
```

## Architecture

### Workspace Crates

```
crates/
├── cli/              # maestro binary - CLI entry point
├── cockpit/          # maestro-cockpit - Ratatui TUI library
├── pi-mono/          # Pi-Mono integration (agent discovery/execution)
├── lsp-bridge/       # LSP protocol bridge
└── ktop_collectors/  # System metrics collection

src/
└── leindex/          # leindex-core - Code analysis engine + memory service
```

### Dependency Rules (STRICT - One-Way)

```
cli → cockpit + leindex-core + pi-mono
cockpit → leindex-core
pi-mono → (standalone)
leindex-core ↛ cockpit (FORBIDDEN)
```

**Never add a dependency from leindex-core to cockpit.** This is enforced by `make policy-check`.

### Key Modules

#### LeIndex Core (`src/leindex/src/`)

- **5-Layer Code Analysis**: AST, Call Graph, CFG, DFG, Slicing
- **Multi-language support**: Python, TypeScript, JavaScript, Rust, Go, Java, C, C++
- **Memory system**: SQLite/Turso backend with vector search (HNSW)
- **Orchestration engine**: Track/task execution with agent runners

Key files:
- `lib.rs` - Module exports
- `memory/` - Memory service, MCP pool, LSP pool, session management
- `orchestrate/` - Conductor engine, rate limiting, agent runners
- `multi_lang_*.rs` - Language-agnostic analyzers

#### Cockpit TUI (`crates/cockpit/src/`)

- `app.rs` - Main TUI application (280KB, central state machine)
- `conductor/` - Track visualization, task execution UI
- `tabs/` - Dashboard, Sessions, Projects, LSP, Memory, ktop tabs
- `theme.rs` - Color scheme and styling

#### CLI (`crates/cli/src/`)

- `main.rs` - Command routing (tui, memory, analyze, etc.)
- Delegates to cockpit and leindex-core

### LeIndex is MANDATORY

**You MUST use LeIndex for code search and analysis in this codebase.**

```bash
# Set the workspace path FIRST
/maestro:leindex set_path "/run/media/scooter/W.D SSD/Prod/maestro"

# Force reindex to ensure up-to-date index
/maestro:leindex force_reindex

# Search code
/maestro:leindex search "authentication"

# Analyze a file
/maestro:leindex analyze src/auth.rs ast callgraph
```

Before implementing changes, use LeIndex to understand the codebase context.

## Conductor Engine

The Conductor module (in cockpit) provides autonomous track execution inspired by Ralph TUI:

- **Track/Task Model**: Parses tracks.md and plan.md
- **Iteration Loop**: Select → Prompt → Run → Detect → Update
- **LeIndex Integration**: Token-efficient context injection
- **Session Persistence**: Lock files, crash recovery

Modes:
- **Planning**: Generate/update plans without implementation
- **Building**: Execute tasks with auto-commit

State directory: `~/.maestro/conductor/`

## Project Structure

```
maestro/
├── product.md          # Product vision
├── tech-stack.md       # Technology choices
├── workflow.md         # Development workflow
├── tracks.md           # Track registry
├── tracks/<track_id>/  # Individual tracks (spec.md, plan.md)
├── skills/             # Maestro skills (109 skills)
├── hooks/              # Session and tool hooks
├── agents/             # Agent definitions
├── memory/             # Nexus Memory System
└── [templates only]    # AI tool templates

src/
└── leindex/            # LeIndex code analysis engine
```

## Testing

Rust tests are colocated with source files using `#[cfg(test)]` modules. Run with `cargo test`.

Python tests (legacy) are in `maestro/*/tests/` directories. Run with `pytest`.

## Code Style

- **Rust**: Standard Rust formatting via `cargo fmt`
- **Line length**: 100 chars for Python (see pyproject.toml)
- **Imports**: Use `cargo fmt` for Rust; isort for Python

## Important Files

- `Makefile` - Build commands and policy checks
- `Cargo.toml` - Workspace definition
- `pyproject.toml` - Python packaging (legacy components)
- `vendor/libsql/` - Vendored libsql dependency

## Vendor Patches

Before building, vendor patches are applied automatically via `scripts/apply-vendor-patches.sh`.
