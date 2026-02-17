# Plan: Maestro Overhaul (IronClaw, ZeroClaw, Moltis Merger)

## Phase 1: Infrastructure & Core Traits
Goal: Define the foundational traits and setup the project structure to support modularity and performance.

- [x] Task: Define Core Traits (Provider, Channel, Memory, Tool, Observer) in `crates/core/src/traits.rs`.
- [x] Task: Implement Unified Config System with XChaCha20-Poly1305 Encryption in `crates/core/src/config.rs`.
- [x] Task: Setup Core-Only Workspace Structure in `crates/core` (no `overhaul` crates).
- [x] Task: Implement Leak Detection & Security Middleware Primitives in `crates/core/src/security.rs`.
- [x] Task: Maestro - User Manual Verification 'Phase 1: Infrastructure' (Verified via `cargo test -p maestro-core`).

## Phase 2: Core Engine & Memory
Goal: Implement an evented async core loop, approval/auth state machine, and Tantivy-backed hybrid memory, using IronClaw/ZeroClaw/Moltis patterns captured in `analysis_foundation_20260217.md`.

- [x] Task: Implement Session/Thread State Machine in `crates/core/src/engine/state.rs` and `crates/core/src/engine/session.rs`. (a21638f)
  - Include explicit states for Processing, AwaitingApproval, AwaitingAuth, Completed, Failed.
  - Add request-id validation semantics for approval submissions (IronClaw parity).
- [x] Task: Implement Async Agent Loop + Intent Routing in `crates/core/src/engine/loop.rs` and `crates/core/src/engine/router.rs`. (f1934d3)
  - Build bounded tool-iteration loop with native-tool and text-fallback handling (ZeroClaw parity).
  - Emit deterministic terminal outcomes (Response / NeedApproval / Error).
- [ ] Task: Implement Tool-Call Parsing + Robust Fallbacks in `crates/core/src/engine/tool_parse.rs`.
  - Support structured tool calls and tagged-text fallback parsing.
  - Add malformed/alias-tag handling and strict non-cross-match behavior.
- [ ] Task: Implement Approval Manager + Policy Hooks in `crates/core/src/security/approval.rs`.
  - Support needs_approval, decision recording, and "always" auto-approve behavior.
  - Preserve channel-aware decision policy entrypoints (CLI-interactive vs non-interactive).
- [ ] Task: Implement Auth Interrupt/Resume Flow in `crates/core/src/engine/auth.rs` and `crates/core/src/engine/loop.rs`.
  - On auth-required tool outcomes: transition thread to AwaitingAuth and short-circuit loop continuation.
  - On token submission: validate, resume execution path, emit completion/failure events.
- [ ] Task: Implement Ordered Event Stream in `crates/core/src/engine/events.rs`.
  - Emit thinking/tool-start/tool-end/delta/retry/final/error events in order (Moltis parity).
  - Ensure event payloads are persistence-safe (size-capped large fields).
- [ ] Task: Implement Context Compaction + Retry-on-Overflow in `crates/core/src/engine/compaction.rs`.
  - Trigger compaction on context-window overflow; retry exactly once.
  - Preserve summary insertion semantics before hard trimming.
- [ ] Task: Implement Tantivy Hybrid Memory Provider in `crates/core/src/memory/tantivy.rs` and `crates/core/src/memory/hybrid.rs`.
  - Replace SQLite-specific hybrid memory assumptions with Tantivy index/query flow.
  - Provide hybrid ranking contract compatible with existing Memory trait consumers.
- [ ] Task: Integrate LeIndex Semantic Graph as Memory Signal in `crates/core/src/memory/leindex_provider.rs`.
  - Fuse graph-aware signals with Tantivy hybrid retrieval.
  - Keep provider boundary explicit so retrieval fusion is testable in isolation.
- [ ] Task: Add Persistence Pipeline for Session/Turn/Tool Events in `crates/core/src/engine/persistence.rs`.
  - Persist user turn, assistant turn, tool results, and reasoning breadcrumbs.
  - Ensure media/large payload references are stored as lightweight pointers.
- [ ] Task: Write Failing Tests First for Phase 2 contracts in:
  - `crates/core/tests/engine_loop.rs`
  - `crates/core/tests/approval_auth.rs`
  - `crates/core/tests/tool_parse.rs`
  - `crates/core/tests/memory_hybrid_tantivy.rs`
  - `crates/core/tests/compaction_retry.rs`
- [ ] Task: Implement Phase 2 code to pass all new tests and preserve deterministic event ordering.
- [ ] Task: Maestro - User Manual Verification 'Phase 2: Core Engine' (Protocol in workflow.md)

## Phase 3: Capabilities & Sandboxing
Goal: Add sub-agents, routines, and secure tool execution.

- [ ] Task: Implement Sub-Agent Delegation Tool (`spawn_agent`).
- [ ] Task: Build Routines Engine (Cron & Event Scheduler).
- [ ] Task: Implement Dual-Tier Sandboxing (WASM + Docker).
- [ ] Task: Integrate MCP Client for external tool support.
- [ ] Task: Maestro - User Manual Verification 'Phase 3: Capabilities' (Protocol in workflow.md)

## Phase 4: Interface & Communication
Goal: Build the Enhanced TUI, Web Dashboard, and Multi-Channel support.

- [ ] Task: Implement High-Performance TUI (Ratatui/Crossterm).
- [ ] Task: Build Axum-based Web Gateway with SSE/WebSocket Streaming.
- [ ] Task: Implement Core Channels (Telegram, Discord, Slack) using Channel trait.
- [ ] Task: Implement Web UI Dashboard with Job Monitoring and Voice Support.
- [ ] Task: Maestro - User Manual Verification 'Phase 4: Interface' (Protocol in workflow.md)

## Phase 5: Integration & Polish
Goal: Final wiring, documentation, and performance verification.

- [ ] Task: Align all features with Maestro Spec-Driven Workflow (Metadata/Tracks integration).
- [ ] Task: Conduct Performance Benchmarking (Target <5MB RAM, <10ms startup).
- [ ] Task: Final "Tzar of Excellence" Review across all modules.
- [ ] Task: Maestro - User Manual Verification 'Phase 5: Integration' (Protocol in workflow.md)
