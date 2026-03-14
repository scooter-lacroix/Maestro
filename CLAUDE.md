# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## LeIndex Tool Priority (MANDATORY)

**LeIndex MCP tools ALWAYS take priority over grep/glob/read/rg and similar tools.**

When exploring or analyzing this codebase:

1. **For semantic code search**: Use `mcp__leindex__leindex_search` instead of Grep
2. **For understanding code structure**: Use `mcp__leindex__leindex_context` instead of Read for symbol exploration
3. **For deep analysis**: Use `mcp__leindex__leindex_deep_analyze` for multi-file understanding
4. **For file discovery**: Use `mcp__leindex__leindex_project_map` instead of Glob for project structure

Only fall back to standard tools (Grep, Glob, Read) if:
- LeIndex returns no results for a valid query
- The MCP server is unavailable
- You need non-code file operations (Bash commands, system operations)

**Example usage:**
```bash
# Semantic search - finds symbols by meaning, not just text
mcp__leindex__leindex_search(query="authentication flow")

# Context expansion - see callers/callees of a function
mcp__leindex__leindex_context(node_id="src/auth.rs:login")

# Deep analysis - understand multi-file relationships
mcp__leindex__leindex_deep_analyze(query="How does the Conductor engine work?")
```

## Common Development Commands

### Rust (Primary Codebase)

```bash
# Build all workspace members
cargo build --workspace

# Build release (optimized, stripped binary)
cargo build --release --workspace

# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p maestro-cli
cargo test -p maestro-cockpit
cargo test -p leindex-core

# Run single test
cargo test test_name --workspace

# Check without building (faster validation)
cargo check --workspace

# Format code
cargo fmt --all

# Run linter
cargo clippy --workspace -- -D warnings

# Install locally
cargo install --path crates/cli
```

### Python (Legacy/Supporting Code)

```bash
# Run Python tests
pytest

# Run with coverage
pytest --cov=maestro --cov-report=html

# Format code
ruff format maestro/

# Lint
ruff check maestro/
```

### TrackLens (React/TypeScript)

```bash
# Install dependencies
bun install

# Build TrackLens editor
bun run build:tracklens

# Development server
cd packages/tracklens-editor && bun run dev
```

## High-Level Architecture

Maestro is a **spec-driven development orchestration framework** with a Rust-first architecture.

### Workspace Structure

```
crates/
├── cli/              # Main "maestro" binary entry point
├── cockpit/          # Ratatui Terminal UI (7 tabs: Dashboard, Sessions, Projects, Analysis, LSP, Memory, Settings)
│   └── src/conductor/   # Track-based workflow execution engine
├── pi-mono/          # Pi-Mono agent detection/discovery/execution
├── lsp-bridge/       # LSP-to-MCP protocol bridge
├── core/             # Engine, security, capabilities, memory backends
├── maestro-claw/     # Agent execution framework (providers, tools, channels)
├── gateway/          # SSE/WebSocket gateway server
└── ktop_collectors/  # System metrics (CPU, disk, etc.)

leindex/rust/         # LeIndex core library (code analysis, storage, migrations)
└── src/migrations/      # Turso/libsql database migrations

src/maestro-tab/      # Forked tab-rs terminal multiplexer (tmux integration)
```

### Dependency Rules (One-Way)

```
cli → cockpit + leindex-core + pi-mono + gateway
cockpit → leindex-core
gateway → maestro-claw + leindex-core
maestro-claw → leindex-core
pi-mono → (standalone)
leindex-core ↛ cockpit (forbidden - prevents circular deps)
```

### Key Systems

1. **LeIndex Core**: 5-layer code analysis with Turso/libsql backend. Provides semantic search, PDG traversal, and LLM-ready context extraction.

2. **Cockpit TUI**: Terminal interface for managing tmux sessions, MCP connections, and track execution. Uses ratatui for rendering.

3. **Conductor Engine**: Token-efficient track automation (inspired by Ralph TUI). Parses tracks.md/plan.md, uses LeIndex for context injection, and executes tasks via subagents.

4. **Pi-Mono**: Agent discovery and execution layer for mono-repo environments. Detects Pi projects, discovers agents, and provides unified execution interface.

5. **Maestro-Claw**: Agent execution framework with multi-provider support (Anthropic, OpenAI, Ollama, OpenRouter), streaming deltas, and typed status handling.

### Database Layer

- **Backend**: Turso/libsql (single file at `~/.maestro/maestro_turso.db`)
- **Migrations**: Version-controlled in `leindex/rust/src/migrations/`
- **Features**: Vector search (DiskANN), LSP state persistence, memory storage

## Development Workflow Principles

From `maestro/workflow.md`:

1. **The Plan is the Source of Truth**: All work tracked in `plan.md`
2. **The Tech Stack is Deliberate**: Changes documented in `tech-stack.md` before implementation
3. **Test-Driven Development**: Write tests before implementation
4. **High Code Coverage**: Aim for >98% coverage (target >80% minimum)
5. **User Experience First**: Prioritize UX in all decisions
6. **Non-Interactive & CI-Aware**: Use `CI=true` for watch-mode tools
7. **LeIndex for Code Exploration**: Mandatory use of LeIndex tools before code changes (see above)

## Code Style Guidelines

Before editing code, read the applicable style guides in `maestro/maestro_code_styleguides/`:
- `general.md` - Universal coding principles
- `rust.md` - Rust-specific conventions
- `python.md` - Python-specific conventions
- `typescript.md` - TypeScript/JavaScript conventions
- Language/framework-specific guides as applicable

**Key principle from general.md**: "Make the next change easy, not just the current change fast. Prefer obvious code over clever code."

## Testing Strategy

- **Unit tests**: Fast logic tests in `tests/` directories within each crate
- **Integration tests**: Cross-crate interaction tests
- **E2E tests**: Complete workflow validation (maestro/maestro/tests/)
- **250+ tests** across the codebase with >98% coverage target for critical paths

## Important Notes

- **No archived code imports**: CI prevents imports from `archive/` directories (legacy Python CLI, Go TUI)
- **Session state**: Stored in `~/.maestro/` (config, database, logs)
- **tmux integration**: Cockpit manages tmux sessions for isolation
- **MCP pooling**: Efficient socket pooling reduces memory usage by 50%+
