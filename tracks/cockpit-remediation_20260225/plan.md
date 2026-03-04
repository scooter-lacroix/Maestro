# Implementation Plan: Cockpit Comprehensive Remediation

**Track:** cockpit-remediation_20260225  
**Last Updated:** 2026-02-25

---

## Phase 1: P0 — Critical Fixes

### Task 1.1: Fix Settings Save Button
- [ ] Fix `Config::save()` in `src/leindex/src/config.rs` to return error when `dirs::config_dir()` returns `None`
- [ ] Fix save handler in `app.rs` (~line 4732) to match on `Result` and show toast success/error
- [ ] Verify with `cargo check --workspace`

### Task 1.2: Fix Compilation Warnings  
- [ ] Add `#[allow(dead_code)]` to serde deserialization struct fields in maestro-claw providers (openai.rs, anthropic.rs, ollama.rs, openrouter.rs)
- [ ] Fix unused variable warnings in gateway/ws.rs with underscore prefixes
- [ ] Verify with `cargo clippy --workspace --all-targets`

## Phase 2: P1 — Tzar Review Remediation

### Task 2.1: IMP-2/IMP-3 — Provider Warmup Token Waste
- [ ] Fix Anthropic `warmup()` in `providers/anthropic.rs` to use `max_tokens: 1` or delegate to `health_check()`
- [ ] Fix OpenRouter `warmup()` in `providers/openrouter.rs` similarly

### Task 2.2: IMP-4 — Async Mutex for MemoryHook
- [ ] Change `std::sync::Mutex` to `tokio::sync::Mutex` in `hooks/builtin/memory.rs`

### Task 2.3: IMP-5 — Session Named Constructor
- [ ] Add `Session::named(title)` constructor to `session/session.rs`

### Task 2.4: OPT-1 — ToolRegistry::list() Allocation
- [ ] Return iterator or cached slice instead of allocating Vec on every call

### Task 2.5: OPT-2 — SSE Carryover Buffer
- [ ] Replace `carry[pos+1..].to_string()` with `drain()` in all 4 providers

### Task 2.6: EDGE-1 — Configurable Summarization Prompt
- [ ] Make agent loop summarization prompt in `agent/loop.rs` configurable via `AgentConfig`

### Task 2.7: EDGE-2 — Preserve System Turn on Trim
- [ ] Fix `trim_old_turns` in `session/thread.rs` to preserve first System turn

### Task 2.8: EDGE-3 — Shell Command Normalization
- [ ] Add normalization for bypass tricks in `tools/builtin/shell.rs`

### Task 2.9: EDGE-4 — Session GC with TTL/LRU
- [ ] Add TTL/LRU eviction to session GC in `gateway/routes.rs`

## Phase 3: P1 — MaesterClaw Keyboard Shortcuts

### Task 3.1: Add State Types
- [ ] Add `MaesterClawFocus` enum to `state/types.rs`
- [ ] Add `MaesterClawAction` enum to `state/types.rs`

### Task 3.2: Create Keybinding Module
- [ ] Create `crates/cockpit/src/maesterclaw/keybindings.rs` with `handle_maesterclaw_key()`
- [ ] Implement all 12 shortcuts for CronJobs, McpServers, Sandbox sections

### Task 3.3: Wire into App Event Loop
- [ ] Add MaesterClaw catch-all in `app.rs` key dispatch
- [ ] Add `maesterclaw_focus` field to `App` struct

### Task 3.4: Add Action Modals
- [ ] Create `crates/cockpit/src/maesterclaw/modals.rs` for CronJob CRUD and MCP server modals

## Phase 4: P2 — Agent Integration & Settings

### Task 4.1: Agent Integration Panel
- [ ] Add `CapabilitiesSection::Agents` variant
- [ ] Create `crates/cockpit/src/maesterclaw/agents.rs`
- [ ] Add agent detection, status panel, launch/configure actions

### Task 4.2: MaesterClaw Settings
- [ ] Extend `Config` struct with new fields
- [ ] Extend `SettingsOption` enum
- [ ] Update settings.rs rendering

### Task 4.3: Conductor Telemetry Bus Wiring
- [ ] Subscribe to `BUS` in app.rs event loop
- [ ] Broadcast events in polling.rs after state transitions

## Phase 5: P3 — Memory Banking & Tree

### Task 5.1: Memory Banking Integration
- [ ] Create `crates/cockpit/src/maesterclaw/memory_bridge.rs`
- [ ] Create `crates/cockpit/src/maesterclaw/memory_events.rs`
- [ ] Hook into session creation and conductor events

### Task 5.2: Memory Tree Dependency Graph
- [ ] Add `MemoryNode`, `MemoryTreeState`, `MemoryViewMode` types
- [ ] Implement tree building and rendering in memory.rs
- [ ] Add expand/collapse, view mode toggle

### Task 5.3: Interval-Based Memory Saving
- [ ] Implement `MemoryAutoBank` service with mpsc channel
- [ ] Wire triggers from conductor, session manager, and agent bridge
- [ ] Add periodic checkpoint timer

---

## Verification
After each phase:
1. `cargo check --workspace`
2. `cargo clippy --workspace --all-targets`
3. `cargo test --workspace`
