# Comprehensive Assessment: IronClaw, ZeroClaw, Moltis, and Maestro

## 1. Project Overviews

### IronClaw
- **Focus**: Secure, multi-channel personal AI assistant.
- **Key Strength**: Defense in depth (WASM sandbox, leak detection, patterns), Routines engine (cron, events), and multi-channel support (Telegram, Slack, Discord).
- **Architecture**: Agent loop with intent routing and parallel job scheduler.

### ZeroClaw
- **Focus**: Ultra-lightweight performance and hardware agnosticism.
- **Key Strength**: <5MB RAM footprint, <10ms startup, trait-based modularity for everything (providers, channels, memory, tools).
- **Architecture**: Pure Rust, trait-based abstraction layers, zero external dependencies for core features (like vector search in SQLite).

### Moltis
- **Focus**: Personal AI gateway with rich local-first features.
- **Key Strength**: Web UI, voice support, hook system (lifecycle events), sub-agent delegation (`spawn_agent`), and MCP integration.
- **Architecture**: Axum-based gateway, workspace-organized crates, robust sandboxing (Docker/Apple).

### Maestro (Current)
- **Focus**: Spec-driven development workflow.
- **Key Strength**: `spec.md` and `plan.md` driven development, TDD emphasis, automatic agent selection, and project memory.
- **Architecture**: Markdown protocols + Rust core (LeIndex).

---

## 2. Core Philosophies to Merge

1. **Efficiency (from ZeroClaw)**: Minimum binary size, minimum RAM usage, and maximum speed. Use traits to allow swapping components without overhead.
2. **Security (from IronClaw & Moltis)**: Sandbox execution (WASM for light tools, Docker for heavy ones), secret redaction, leak detection, and strictly scoped filesystem access.
3. **Extensibility (from ZeroClaw & Moltis)**: Trait-based subsystems and a robust hook system for lifecycle events.
4. **Local-First (Common to all)**: Your data stays yours. Local persistence (SQLite/PostgreSQL), local indexing, and local execution where possible.
5. **Spec-Driven (from Maestro)**: Every action is planned and specified. The "Plan is the Source of Truth".

---

## 3. Proposed Unified Architecture ("Maestro Overhaul")

### A. Core Runtime (Rust)
- **Engine**: A unified agent loop that uses traits for pluggable providers (LLMs), channels, and tools.
- **Performance**: Static binaries, async/await for concurrency, and minimal dependencies.
- **Memory Management**: Custom SQLite-based hybrid search (ZeroClaw style) but with LeIndex's semantic graph depth.

### B. Security Layer
- **Sandbox**: Dual-tier sandboxing. WASM for lightweight, safe tools; Docker for environment-heavy tasks.
- **Middleware**: Leak detection and secret redaction on all I/O boundaries.

### C. Workflow Integration
- **Maestro Protocols**: Native support for reading/writing `spec.md`, `plan.md`, and `tech-stack.md` as core assistant behaviors.
- **Automatic Agent Selection**: Internal "Router" (IronClaw style) that classifies tasks and selects the best "Agent Identity" (Maestro agents) for the job.

### D. Capabilities
- **MCP**: Native Model Context Protocol support for extending capabilities without code changes.
- **Routines**: Background execution for long-running tasks or periodic checks.
- **Hooks**: Strategic integration points (Maestro's `Critical Think` integrated into the agent's lifecycle).

---

## 4. Implementation Priorities

1. **Rust Core Refactor**: Unify the `maestro` Rust core with the trait-based modularity of `ZeroClaw`.
2. **Security Hardening**: Implement the leak detection and WASM sandboxing from `IronClaw`.
3. **UI/UX Enhancement**: Integrate the web gateway and streaming capabilities from `Moltis`.
4. **Protocol Alignment**: Ensure all "Claw" features respect and reinforce the Maestro spec-driven workflow.
